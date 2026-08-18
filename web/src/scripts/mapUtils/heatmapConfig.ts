// Tweakable visual configuration for the map representation
export const HEATMAP_CONFIG = {
  // The zoom level where point circles begin to show up
  pointMinZoom: 13,
  heatmapMaxZoom: 16,

  // Heatmap Intensity: global density multiplier by zoom.
  // Kept low when zoomed out to show structure, ramped up as points separate.
  intensity: [
    [0, 0.2], // World view – gentle presence
    [5, 0.35], // Continent scale
    [10, 0.7], // Regional clusters become distinct
    [14, 1.0], // Local peaks stand out clearly
  ],

  // Heatmap Radius: blending radius in pixels per zoom.
  // A smooth decay that prevents oceans from flooding while keeping cities connected.
  radius: [
    [0, 12], // Tight at global level to avoid ocean smearing
    [5, 18], // Merges distant points into corridors
    [10, 30], // Natural separation of neighbourhoods
    [16, 25], // Crisp individual hotspots before fading
  ],

  // Heatmap Opacity: seamless crossover from heatmap to point markers.
  opacity: [
    [12, 0.75],
    [16, 0],
  ],

  // Color Stops: classic thermal gradient (transparent → cold → hot → peak white).
  // Uses solid colours so that layer blending is controlled only by heatmap-opacity.
  colorStops: [
    [0, 'rgba(0, 0, 0, 0)'], // fully invisible boundary
    [0.05, 'rgb(76 70 184 / 0.7)'], // deep indigo for lowest density
    [0.2, 'rgba(0, 140, 200, 0.7)'], // rich cyan/blue
    [0.4, 'rgba(40, 200, 100, 0.7)'], // vivid green
    [0.6, 'rgba(240, 220, 40, 0.7)'], // warm yellow
    [0.8, 'rgb(214 116 49 / 0.7)'], // intense orange
    [1, 'rgb(213 75 75 / 0.7)'], // bright near-white for extreme peaks
  ],

  // Point markers – subtle circles at high zoom to show exact location.
  point: {
    color: 'rgb(80, 30, 120)', // deep purple, visible on most maps
    strokeColor: '#ffffff',
    strokeWidth: 1.8,
    radius: [
      [13, 4],
      [17, 11],
    ],
    opacity: [
      [13, 0],
      [15.5, 0.8],
      [16, 1],
    ],
  },
}
