package dev.ruurd.photos.testing

import dev.ruurd.photos.core.common.AppDispatchers
import kotlinx.coroutines.CoroutineDispatcher

/** All three dispatchers on one test dispatcher so use cases stay deterministic. */
class TestAppDispatchers(
    private val dispatcher: CoroutineDispatcher
) : AppDispatchers {
    override val default: CoroutineDispatcher = dispatcher
    override val io: CoroutineDispatcher = dispatcher
    override val main: CoroutineDispatcher = dispatcher
}
