package dev.ruurd.photos.features.timeline

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.ruurd.photos.core.common.AppDispatchers
import dev.ruurd.photos.domain.model.TimelineLayoutIndex
import dev.ruurd.photos.domain.usecase.CalculateTimelineLayoutUseCase
import dev.ruurd.photos.domain.usecase.ObserveTimelineUseCase
import dev.ruurd.photos.domain.usecase.TimelineLayoutParams
import kotlinx.collections.immutable.ImmutableMap
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.stateIn

data class TimelineUiState(
    val isLoading: Boolean = true,
    val layout: TimelineLayoutIndex = TimelineLayoutIndex(),
)

class TimelineViewModel(
    private val observeTimelineUseCase: ObserveTimelineUseCase,
    private val calculateTimelineLayoutUseCase: CalculateTimelineLayoutUseCase,
    private val dispatchers: AppDispatchers,
    isBootstrapping: Flow<Boolean>,
    val imageRevisions: StateFlow<ImmutableMap<String, Int>> = MutableStateFlow(persistentMapOf()),
) : ViewModel() {

    private val viewport = MutableStateFlow(Viewport())

    @OptIn(ExperimentalCoroutinesApi::class)
    val uiState: StateFlow<TimelineUiState> = isBootstrapping
        .flatMapLatest { bootstrapping ->
            if (bootstrapping) {
                flowOf(TimelineUiState(isLoading = true))
            } else {
                combine(
                    observeTimelineUseCase().distinctUntilChanged(),
                    viewport,
                ) { media, viewport ->
                    if (viewport.widthPx <= 0) {
                        TimelineUiState(isLoading = true)
                    } else {
                        TimelineUiState(
                            isLoading = false,
                            layout = calculateTimelineLayoutUseCase(
                                media = media,
                                params = TimelineLayoutParams(
                                    containerWidthPx = viewport.widthPx,
                                    density = viewport.density,
                                    desiredRowHeightDp = desiredRowHeightDp(
                                        viewport.widthPx / viewport.density,
                                    ),
                                ),
                            ),
                        )
                    }
                }
            }
        }
        .flowOn(dispatchers.default)
        .stateIn(viewModelScope, SharingStarted.Lazily, TimelineUiState())

    fun onViewportChanged(widthPx: Int, density: Float) {
        if (widthPx <= 0 || density <= 0f) return
        val current = viewport.value
        if (current.widthPx != widthPx || current.density != density) {
            viewport.value = Viewport(widthPx, density)
        }
    }

    private data class Viewport(val widthPx: Int = 0, val density: Float = 1f)

    companion object {
        // Taller rows once the window is tablet/landscape wide, mirroring the web timeline.
        private const val ROW_HEIGHT_PHONE_DP = 180f
        private const val ROW_HEIGHT_WIDE_DP = 240f
        private const val WIDE_WIDTH_DP = 600f

        private fun desiredRowHeightDp(widthDp: Float): Float = if (widthDp >= WIDE_WIDTH_DP) ROW_HEIGHT_WIDE_DP else ROW_HEIGHT_PHONE_DP
    }
}
