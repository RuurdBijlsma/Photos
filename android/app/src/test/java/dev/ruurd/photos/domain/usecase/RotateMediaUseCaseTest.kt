package dev.ruurd.photos.domain.usecase

import dev.ruurd.photos.core.database.entity.MediaEntity
import dev.ruurd.photos.core.database.entity.SyncStatus
import dev.ruurd.photos.core.database.entity.toTimelineProjection
import dev.ruurd.photos.core.image.FullResFileStore
import dev.ruurd.photos.core.network.dto.UpdateMediaItemRequestDto
import dev.ruurd.photos.data.datasource.local.LocalFileOverwriteHelper
import dev.ruurd.photos.domain.model.MediaDetails
import dev.ruurd.photos.domain.repository.MediaMutationRepository
import dev.ruurd.photos.testing.FakeMediaRepository
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.io.IOException

class RotateMediaUseCaseTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    @Test
    fun `cloud-only rotate warms the new original before swapping the Room row`() = runTest {
        val repository = FakeMediaRepository()
        repository.upsert(
            MediaEntity(
                id = "cloud_old",
                cloudId = "old",
                ratio = 2f,
                sizeBytes = 100L,
                syncStatus = SyncStatus.CLOUD_ONLY,
                hasThumbnails = true,
            ),
        )
        val mutations = FakeMediaMutationRepository(newCloudId = "new")
        val warmedFile = tempFolder.newFile("rotated.jpg").apply { writeBytes(ByteArray(256)) }
        val fullRes = FakeFullResFileStore(filesByCloudId = mapOf("new" to warmedFile))
        val events = mutableListOf<String>()
        val useCase = RotateMediaUseCase(
            mediaRepository = repository,
            mediaMutationRepository = mutations,
            localFileOverwriteHelper = FakeLocalFileOverwriteHelper(),
            fullResFileCache = fullRes,
        )

        val outcome = useCase.commit(
            item = repository.getById("cloud_old")!!.toTimelineProjection(),
            details = MediaDetails(cloudId = "old", fileName = "one.jpg"),
            targetOrientation = 6,
            newRatio = 0.5f,
            onIdRedirect = { oldId, newId -> events += "redirect:$oldId->$newId" },
        ).getOrThrow()

        assertEquals(6, mutations.lastOrientation)
        assertEquals("cloud_old", outcome.oldMediaId)
        assertEquals("cloud_new", outcome.newMediaId)
        assertEquals("new", outcome.newCloudId)
        assertEquals(warmedFile, outcome.warmedFullResFile)
        assertEquals(256L, outcome.updatedSizeBytes)
        assertEquals(listOf("redirect:cloud_old->cloud_new"), events)
        assertNull(repository.getById("cloud_old"))

        val row = repository.getById("cloud_new")!!
        assertEquals("new", row.cloudId)
        assertEquals(0.5f, row.ratio)
        assertEquals(256L, row.sizeBytes)
        assertFalse(row.hasThumbnails)
        assertEquals(listOf("old"), fullRes.evictedCloudIds)
        assertEquals(listOf("new"), fullRes.requestedCloudIds)
    }

    @Test
    fun `cloud-only rotate still commits the Room swap when warm-up fails`() = runTest {
        val repository = FakeMediaRepository()
        repository.upsert(
            MediaEntity(
                id = "cloud_old",
                cloudId = "old",
                ratio = 2f,
                syncStatus = SyncStatus.CLOUD_ONLY,
                hasThumbnails = true,
            ),
        )
        val useCase = RotateMediaUseCase(
            mediaRepository = repository,
            mediaMutationRepository = FakeMediaMutationRepository(newCloudId = "new"),
            localFileOverwriteHelper = FakeLocalFileOverwriteHelper(),
            fullResFileCache = FakeFullResFileStore(),
        )

        val outcome = useCase.commit(
            item = repository.getById("cloud_old")!!.toTimelineProjection(),
            details = MediaDetails(cloudId = "old", fileName = "one.jpg"),
            targetOrientation = 6,
            newRatio = 0.5f,
        ).getOrThrow()

        assertNull(outcome.warmedFullResFile)
        assertNull(repository.getById("cloud_old"))
        assertEquals("new", repository.getById("cloud_new")?.cloudId)
    }

    @Test
    fun `synced rotate adopts the new cloud id, drops the duplicate row and records the new local size`() = runTest {
        val repository = FakeMediaRepository()
        repository.upsert(
            MediaEntity(
                id = "local_1",
                cloudId = "old",
                localContentUri = "content://media/1",
                ratio = 1f,
                sizeBytes = 100L,
                syncStatus = SyncStatus.SYNCED,
            ),
        )
        // A sync racing the PUT already inserted the new cloud row.
        repository.upsert(
            MediaEntity(id = "cloud_new", cloudId = "new", syncStatus = SyncStatus.CLOUD_ONLY),
        )
        val overwrite = FakeLocalFileOverwriteHelper(overwriteSize = 1500L)
        val useCase = RotateMediaUseCase(
            mediaRepository = repository,
            mediaMutationRepository = FakeMediaMutationRepository(newCloudId = "new"),
            localFileOverwriteHelper = overwrite,
            fullResFileCache = FakeFullResFileStore(),
        )

        val outcome = useCase.commit(
            item = repository.getById("local_1")!!.toTimelineProjection(),
            details = MediaDetails(cloudId = "old", fileName = "one.jpg", sizeBytes = 999L),
            targetOrientation = 8,
            newRatio = 0.5f,
        ).getOrThrow()

        assertEquals("local_1", outcome.newMediaId)
        assertTrue(outcome.localOverwriteSucceeded)
        assertEquals(1500L, outcome.updatedSizeBytes)
        assertNull(repository.getById("cloud_new"))

        val row = repository.getById("local_1")!!
        assertEquals("new", row.cloudId)
        assertEquals(0.5f, row.ratio)
        // Must use the overwritten file's actual size rather than stale pre-rotation details.sizeBytes.
        assertEquals(1500L, row.sizeBytes)
        assertFalse(row.hasThumbnails)
        assertFalse(row.autoBackupSuppressed)
        assertEquals("content://media/1", overwrite.overwrittenUri)
    }

    @Test
    fun `local-only rotate writes the EXIF orientation and the new size`() = runTest {
        val repository = FakeMediaRepository()
        repository.upsert(
            MediaEntity(
                id = "local_1",
                localContentUri = "content://media/1",
                ratio = 1f,
                sizeBytes = 100L,
                syncStatus = SyncStatus.LOCAL_ONLY,
            ),
        )
        val overwrite = FakeLocalFileOverwriteHelper(orientationSize = 1234L)
        val useCase = RotateMediaUseCase(
            mediaRepository = repository,
            mediaMutationRepository = FakeMediaMutationRepository(),
            localFileOverwriteHelper = overwrite,
            fullResFileCache = FakeFullResFileStore(),
        )

        val outcome = useCase.commit(
            item = repository.getById("local_1")!!.toTimelineProjection(),
            details = null,
            targetOrientation = 8,
            newRatio = 2f,
        ).getOrThrow()

        assertEquals(8, overwrite.writtenOrientation)
        assertNull(outcome.newCloudId)
        val row = repository.getById("local_1")!!
        assertEquals(2f, row.ratio)
        assertEquals(1234L, row.sizeBytes)
    }

    @Test
    fun `a failed PUT leaves the row untouched`() = runTest {
        val repository = FakeMediaRepository()
        repository.upsert(
            MediaEntity(id = "cloud_old", cloudId = "old", syncStatus = SyncStatus.CLOUD_ONLY),
        )
        val useCase = RotateMediaUseCase(
            mediaRepository = repository,
            mediaMutationRepository = FakeMediaMutationRepository(newCloudId = null),
            localFileOverwriteHelper = FakeLocalFileOverwriteHelper(),
            fullResFileCache = FakeFullResFileStore(),
        )

        val result = useCase.commit(
            item = repository.getById("cloud_old")!!.toTimelineProjection(),
            details = MediaDetails(cloudId = "old"),
            targetOrientation = 6,
            newRatio = 0.5f,
        )

        assertTrue(result.isFailure)
        assertEquals("old", repository.getById("cloud_old")?.cloudId)
    }
}

private class FakeMediaMutationRepository(private val newCloudId: String? = null) : MediaMutationRepository {
    var lastOrientation: Int? = null

    override suspend fun updateItem(
        cloudId: String,
        request: UpdateMediaItemRequestDto,
    ): Result<String> {
        lastOrientation = request.orientation
        return newCloudId?.let { Result.success(it) } ?: Result.failure(IOException("offline"))
    }

    override suspend fun reprocess(cloudId: String): Result<Unit> = Result.success(Unit)
}

private class FakeLocalFileOverwriteHelper(
    private val orientationSize: Long? = 100L,
    private val overwriteSize: Long? = 100L,
) : LocalFileOverwriteHelper {
    var writtenOrientation: Int? = null
    var overwrittenUri: String? = null

    override suspend fun writeOrientation(contentUri: String, orientation: Int): Long? {
        writtenOrientation = orientation
        return orientationSize
    }

    override suspend fun overwriteFromCloud(
        cloudId: String,
        contentUri: String,
        fileName: String?,
    ): Long? {
        overwrittenUri = contentUri
        return overwriteSize
    }
}

private class FakeFullResFileStore(
    private val filesByCloudId: Map<String, File> = emptyMap(),
) : FullResFileStore {
    val requestedCloudIds = mutableListOf<String>()
    val evictedCloudIds = mutableListOf<String>()

    override fun peek(cloudId: String): File? = filesByCloudId[cloudId]

    override suspend fun get(cloudId: String, fileName: String?): File? {
        requestedCloudIds += cloudId
        return filesByCloudId[cloudId]
    }

    override suspend fun evict(cloudId: String) {
        evictedCloudIds += cloudId
    }

    override suspend fun clear() = Unit
}
