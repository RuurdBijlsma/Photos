package dev.ruurd.photos.core.common

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

/**
 * Runs at most one [producer] per key at a time: callers that arrive while a load is running await
 * the same [Deferred] instead of starting a second one. The load is cancelled once its last waiter
 * leaves or [clear] is called.
 *
 * [generation] is captured before a load starts so a producer can discard a result that [clear]
 * invalidated in the meantime.
 */
class SingleFlightGroup<K : Any, V>(
    private val scope: CoroutineScope,
    private val dispatcher: CoroutineDispatcher,
) {
    private val generationCounter = AtomicLong(0L)
    private val inFlight = ConcurrentHashMap<K, Entry<V>>()

    val generation: Long get() = generationCounter.get()

    /** Result of the shared load, or `null` when it failed or was cancelled. */
    suspend fun load(key: K, producer: suspend (K) -> V): V? {
        val expectedGeneration = generationCounter.get()
        val entry = inFlight.compute(key) { id, existing ->
            val active = existing?.takeIf {
                it.generation == expectedGeneration && !it.job.isCancelled && !it.job.isCompleted
            } ?: Entry(
                generation = expectedGeneration,
                job = scope.async(dispatcher, start = CoroutineStart.LAZY) { producer(id) },
            ).also { created ->
                created.job.invokeOnCompletion { inFlight.remove(id, created) }
            }
            active.waiters.incrementAndGet()
            active
        }!!

        return try {
            entry.job.await()
        } catch (_: CancellationException) {
            // Only rethrow if this caller's own coroutine was cancelled; a cancelled shared load
            // returns null without cancelling unrelated callers.
            currentCoroutineContext().ensureActive()
            null
        } finally {
            if (entry.waiters.decrementAndGet() <= 0) {
                inFlight.remove(key, entry)
                entry.job.cancel()
            }
        }
    }

    /** Cancels the load for [key], if one is running. */
    fun cancel(key: K) {
        inFlight.remove(key)?.job?.cancel()
    }

    /** Cancels every running load and invalidates results produced from now on. */
    fun clear() {
        generationCounter.incrementAndGet()
        val pending = inFlight.values.toList()
        inFlight.clear()
        pending.forEach { it.job.cancel() }
    }

    private class Entry<V>(
        val generation: Long,
        val job: Deferred<V>,
    ) {
        val waiters = AtomicInteger(0)
    }
}
