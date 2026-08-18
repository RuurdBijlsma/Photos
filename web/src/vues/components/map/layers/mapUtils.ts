import type { GeoJSONFeature } from 'maplibre-gl'
import type * as GeoJSON from 'geojson'
import type {
  MapPhotoItem,
  MapPhotosResponse,
  SimpleTimelineItem,
} from '@/scripts/types/generated/timeline.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { getThumbnailHeight } from '@/scripts/utils.ts'

export function getLngLat(item: MapPhotoItem): [number, number] {
  return [item.longitude, item.latitude]
}

export function getFeatureCoordinates(feature: GeoJSONFeature): [number, number] {
  return (feature.geometry as GeoJSON.Point).coordinates as [number, number]
}

export function getItemFromProperties(
  properties: GeoJSONFeature['properties'] | null | undefined,
): SimpleTimelineItem | undefined {
  if (!properties?.id) return undefined
  const ratio = Number(properties.ratio)
  const durationMs = Number(properties.durationMs)
  return {
    id: String(properties.id),
    isVideo: Boolean(properties.isVideo),
    hasThumbnails: Boolean(properties.hasThumbnails),
    ...(Number.isFinite(durationMs) && durationMs > 0 ? { durationMs } : {}),
    ratio: Number.isFinite(ratio) && ratio > 0 ? ratio : 1,
  }
}

export function createPhotosGeoJson(
  photos: MapPhotosResponse,
): GeoJSON.FeatureCollection<GeoJSON.Point> {
  return {
    type: 'FeatureCollection',
    features: photos.items.map((p) => ({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: getLngLat(p),
      },
      properties: {
        id: p.item?.id,
        hasThumbnails: p.item?.hasThumbnails,
        isVideo: p.item?.isVideo,
        durationMs: p.item?.durationMs,
        ratio: p.item?.ratio,
      },
    })),
  }
}

export function getThumbnailUrl(item: SimpleTimelineItem, markerHeight: number) {
  return mediaItemService.getPhotoThumbnail(
    item.id,
    getThumbnailHeight(markerHeight),
    !item.hasThumbnails,
  )
}
