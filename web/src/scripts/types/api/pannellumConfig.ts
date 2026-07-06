export interface PannellumConfig {
  // General options
  /**
   * Specifies the panorama type. Defaults to 'equirectangular'.
   */
  type?: 'equirectangular' | 'cubemap' | 'multires'

  /**
   * If set, the value is displayed as the panorama's title.
   */
  title?: string

  /**
   * If set, the value is displayed as the panorama's author.
   */
  author?: string

  /**
   * If set, the displayed author text is hyperlinked to this URL.
   * The author parameter must also be set for this parameter to have an effect.
   */
  authorURL?: string

  /**
   * Allows user-facing strings to be changed / translated.
   */
  strings?: Record<string, string>

  /**
   * Specifies a base path to load the images from.
   */
  basePath?: string

  /**
   * When set to true, the panorama will automatically load. Defaults to false.
   */
  autoLoad?: boolean

  /**
   * Setting this parameter causes the panorama to automatically rotate when loaded.
   * The value specifies the rotation speed in degrees per second.
   * Positive is counter-clockwise, and negative is clockwise.
   */
  autoRotate?: number

  /**
   * Sets the delay, in milliseconds, to start automatically rotating the panorama
   * after user activity ceases. Only has an effect if autoRotate is set.
   */
  autoRotateInactivityDelay?: number

  /**
   * Sets the delay, in milliseconds, to stop automatically rotating the panorama
   * after it is loaded. Only has an effect if autoRotate is set.
   */
  autoRotateStopDelay?: number

  /**
   * A URL for a fallback viewer in case Pannellum is not supported by the device.
   */
  fallback?: string

  /**
   * If set to true, device orientation control will be used when the panorama is loaded,
   * if the device supports it. Defaults to false.
   */
  orientationOnByDefault?: boolean

  /**
   * If set to false, the zoom controls will not be displayed. Defaults to true.
   */
  showZoomCtrl?: boolean

  /**
   * If set to false, zooming with keyboard will be disabled. Defaults to true.
   */
  keyboardZoom?: boolean

  /**
   * If set to false, zooming with mouse wheel will be disabled. Defaults to true.
   * Can also be set to 'fullscreenonly'.
   */
  mouseZoom?: boolean | 'fullscreenonly'

  /**
   * If set to false, mouse and touch dragging is disabled. Defaults to true.
   */
  draggable?: boolean

  /**
   * Controls the "friction" that slows down the viewer motion after it is dragged and released.
   * Should be in the range (0.0, 1.0]. Defaults to 0.15.
   */
  friction?: number

  /**
   * If set to true, keyboard controls are disabled. Defaults to false.
   */
  disableKeyboardCtrl?: boolean

  /**
   * If set to false, the fullscreen control will not be displayed. Defaults to true.
   */
  showFullscreenCtrl?: boolean

  /**
   * If set to false, no controls are displayed. Defaults to true.
   */
  showControls?: boolean

  /**
   * Adjusts panning speed from touch inputs. Defaults to 1.
   */
  touchPanSpeedCoeffFactor?: number

  /**
   * Sets the panorama's starting yaw position in degrees. Defaults to 0.
   */
  yaw?: number

  /**
   * Sets the panorama's starting pitch position in degrees. Defaults to 0.
   */
  pitch?: number

  /**
   * Sets the panorama's starting horizontal field of view in degrees. Defaults to 100.
   */
  hfov?: number

  /**
   * Sets the minimum yaw the viewer edge can be at, in degrees. Defaults to -180.
   */
  minYaw?: number

  /**
   * Sets the maximum yaw the viewer edge can be at, in degrees. Defaults to 180.
   */
  maxYaw?: number

  /**
   * Sets the minimum pitch the viewer edge can be at, in degrees. Defaults to undefined.
   */
  minPitch?: number

  /**
   * Sets the maximum pitch the viewer edge can be at, in degrees. Defaults to undefined.
   */
  maxPitch?: number

  /**
   * Sets the minimum horizontal field of view, in degrees, that the viewer can be set to.
   * Defaults to 50.
   */
  minHfov?: number

  /**
   * Sets the maximum horizontal field of view, in degrees, that the viewer can be set to.
   * Defaults to 120.
   */
  maxHfov?: number

  /**
   * When set to false, the minHfov parameter is ignored for multires panoramas;
   * an automatically calculated minimum horizontal field of view is used instead.
   * Defaults to false.
   */
  multiResMinHfov?: boolean

  /**
   * If true, a compass is displayed. Defaults to false.
   */
  compass?: boolean

  /**
   * Sets the offset, in degrees, of the center of the panorama from North.
   * Only has an effect if compass is set to true.
   */
  northOffset?: number

  /**
   * Specifies a URL for a preview image to display before the panorama is loaded.
   */
  preview?: string

  /**
   * Specifies the title to be displayed while the load button is displayed.
   */
  previewTitle?: string

  /**
   * Specifies the author to be displayed while the load button is displayed.
   */
  previewAuthor?: string

  /**
   * Specifies pitch of image horizon, in degrees, for correcting non-leveled panoramas.
   */
  horizonPitch?: number

  /**
   * Specifies roll of image horizon, in degrees, for correcting non-leveled panoramas.
   */
  horizonRoll?: number

  /**
   * [API only] Specifies a timing function to be used for animating movements.
   * Function takes a number [0, 1] as its argument and returns a number [0, 1].
   */
  animationTimingFunction?: (time: number) => number

  /**
   * When true, HTML is escaped from configuration strings to help mitigate possible DOM XSS attacks.
   * Defaults to false when using the API.
   */
  escapeHTML?: boolean

  /**
   * Specifies the type of CORS request used. Defaults to 'anonymous'.
   */
  crossOrigin?: 'anonymous' | 'use-credentials'

  /**
   * Specifies an array of hot spots.
   */
  hotSpots?: HotSpotConfig[]

  /**
   * When true, the mouse pointer's pitch and yaw are logged to the console on click.
   * Defaults to false.
   */
  hotSpotDebug?: boolean

  /**
   * Specifies the fade duration, in milliseconds, when transitioning between scenes.
   * Only applicable for tours on the WebGL renderer.
   */
  sceneFadeDuration?: number

  /**
   * Specifies the key numbers that are captured in key events.
   */
  capturedKeyNumbers?: number[]

  /**
   * Specifies an array containing RGB values [0, 1] that sets the background color
   * for areas where no image data is available. Defaults to [0, 0, 0].
   */
  backgroundColor?: [number, number, number]

  /**
   * If set to true, prevent displaying out-of-range areas of a partial panorama.
   * Defaults to false.
   */
  avoidShowingBackground?: boolean

  // Equirectangular specific options
  /**
   * Sets the URL to the equirectangular panorama image.
   */
  panorama?: string

  /**
   * Sets the panorama's horizontal angle of view, in degrees. Defaults to 360.
   */
  haov?: number

  /**
   * Sets the panorama's vertical angle of view, in degrees. Defaults to 180.
   */
  vaov?: number

  /**
   * Sets the vertical offset of the center of the equirectangular image from the horizon, in degrees.
   * Defaults to 0.
   */
  vOffset?: number

  /**
   * If set to true, any embedded Photo Sphere XMP data will be ignored. Defaults to false.
   */
  ignoreGPanoXMP?: boolean

  // Cubemap specific options
  /**
   * An array of URLs for the six cube faces: front, right, back, left, up, down.
   * Partial cubemap images may be specified by giving null instead of a URL.
   */
  cubeMap?: [
    string | null, // front
    string | null, // right
    string | null, // back
    string | null, // left
    string | null, // up
    string | null, // down
  ]

  // Multires specific options
  /**
   * Contains information about the multiresolution panorama.
   */
  multiRes?: MultiResConfig

  // Dynamic content specific options
  /**
   * The panorama source is considered dynamic when set to true. Defaults to false.
   */
  dynamic?: boolean

  /**
   * For dynamic content, viewer will start automatically updating when set to true. Defaults to false.
   */
  dynamicUpdate?: boolean
}

export interface HotSpotConfig {
  /**
   * Specifies the pitch portion of the hot spot's location, in degrees.
   */
  pitch?: number

  /**
   * Specifies the yaw portion of the hot spot's location, in degrees.
   */
  yaw?: number

  /**
   * Specifies the type of the hot spot. Can be 'scene' or 'info'.
   */
  type?: 'scene' | 'info'

  /**
   * Specifies the text displayed when the user hovers over the hot spot.
   */
  text?: string

  /**
   * If specified for an info hot spot, the hot spot links to the specified URL.
   */
  URL?: string

  /**
   * Specifies URL's link attributes. If not set, target is set to '_blank'.
   */
  attributes?: Record<string, string>

  /**
   * Specifies the ID of the scene to link to for scene hot spots.
   */
  sceneId?: string

  /**
   * Specifies the pitch of the target scene in degrees, or 'same'.
   */
  targetPitch?: number | 'same'

  /**
   * Specifies the yaw of the target scene in degrees, or 'same', or 'sameAzimuth'.
   */
  targetYaw?: number | 'same' | 'sameAzimuth'

  /**
   * Specifies the HFOV of the target scene in degrees, or 'same'.
   */
  targetHfov?: number | 'same'

  /**
   * Specifies hot spot ID, for use with API's removeHotSpot function.
   */
  id?: string | number

  /**
   * CSS class used for the hot spot instead of the default CSS classes.
   */
  cssClass?: string

  /**
   * When true, the hot spot is scaled to match changes in the field of view.
   * Defaults to false.
   */
  scale?: boolean
}

export interface MultiResConfig {
  /**
   * The base path of the URLs for the multiresolution tiles.
   */
  basePath?: string

  /**
   * Format string for the location of the tiles relative to multiRes.basePath.
   * Parameters: %l (zoom level), %s (cube face), %x (x index), %y (y index).
   */
  path?: string

  /**
   * Format string for the location of the fallback tiles if WebGL is not supported.
   * Parameter: %s (cube face).
   */
  fallbackPath?: string

  /**
   * Specifies the tiles' file extension without the preceding dot.
   */
  extension?: string

  /**
   * Specifies the size in pixels of each image tile.
   */
  tileResolution?: number

  /**
   * Specifies the maximum zoom level.
   */
  maxLevel?: number

  /**
   * Specifies the size in pixels of the full resolution cube faces.
   */
  cubeResolution?: number
}
