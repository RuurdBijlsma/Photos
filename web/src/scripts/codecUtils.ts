export interface CodecInfo {
  friendlyName: string
  codec: string
  category:
    | 'Modern'
    | 'Apple ProRes'
    | 'MPEG'
    | 'Legacy/QuickTime'
    | 'Professional'
    | 'Web/Streaming'
}

/**
 * A map of common CompressorID / FourCC codes to friendly names.
 * Keys containing trailing spaces (e.g., 'rle ') are padded to preserve the 4-character standard.
 */
export const COMPRESSOR_ID_MAP: Record<string, CodecInfo> = {
  // --- Modern Codecs ---
  avc1: { friendlyName: 'H.264 AVC', codec: 'MPEG-4 Part 10 / AVC', category: 'Modern' },
  avc3: { friendlyName: 'H.264 AVC (in-band SPS/PPS)', codec: 'AVC', category: 'Modern' },
  hvc1: { friendlyName: 'H.265 HEVC', codec: 'HEVC', category: 'Modern' },
  hev1: { friendlyName: 'H.265 HEVC (alternate signaling)', codec: 'HEVC', category: 'Modern' },
  av01: { friendlyName: 'AV1', codec: 'AOMedia AV1', category: 'Modern' },
  vp08: { friendlyName: 'VP8', codec: 'Google VP8', category: 'Modern' },
  vp09: { friendlyName: 'VP9', codec: 'Google VP9', category: 'Modern' },
  dvhe: { friendlyName: 'Dolby Vision over HEVC', codec: 'Dolby Vision', category: 'Modern' },
  dvh1: { friendlyName: 'Dolby Vision', codec: 'Dolby Vision', category: 'Modern' },
  vvc1: { friendlyName: 'H.266 VVC', codec: 'Versatile Video Coding', category: 'Modern' },
  vvi1: { friendlyName: 'H.266 VVC variant', codec: 'VVC', category: 'Modern' },

  // --- Apple / ProRes ---
  apcn: { friendlyName: 'Apple ProRes 422 Standard', codec: 'ProRes', category: 'Apple ProRes' },
  apcs: { friendlyName: 'Apple ProRes 422 LT', codec: 'ProRes', category: 'Apple ProRes' },
  apco: { friendlyName: 'Apple ProRes 422 Proxy', codec: 'ProRes', category: 'Apple ProRes' },
  ap4h: { friendlyName: 'Apple ProRes 4444', codec: 'ProRes', category: 'Apple ProRes' },
  ap4x: { friendlyName: 'Apple ProRes 4444 XQ', codec: 'ProRes', category: 'Apple ProRes' },
  apch: { friendlyName: 'Apple ProRes 422 HQ', codec: 'ProRes', category: 'Apple ProRes' },

  // --- MPEG Family ---
  mp4v: { friendlyName: 'MPEG-4 Part 2', codec: 'MPEG-4', category: 'MPEG' },
  mp2v: { friendlyName: 'MPEG-2 Video', codec: 'MPEG-2', category: 'MPEG' },
  mpeg: { friendlyName: 'MPEG Video', codec: 'MPEG', category: 'MPEG' },
  m1v1: { friendlyName: 'MPEG-1 Video', codec: 'MPEG-1', category: 'MPEG' },

  // --- Legacy / Older QuickTime Codecs ---
  jpeg: { friendlyName: 'Motion JPEG', codec: 'M-JPEG', category: 'Legacy/QuickTime' },
  mjpa: { friendlyName: 'Motion JPEG A', codec: 'M-JPEG', category: 'Legacy/QuickTime' },
  mjpb: { friendlyName: 'Motion JPEG B', codec: 'M-JPEG', category: 'Legacy/QuickTime' },
  SVQ1: { friendlyName: 'Sorenson Video 1', codec: 'Sorenson', category: 'Legacy/QuickTime' },
  SVQ3: { friendlyName: 'Sorenson Video 3', codec: 'Sorenson', category: 'Legacy/QuickTime' },
  cvid: { friendlyName: 'Cinepak', codec: 'Cinepak', category: 'Legacy/QuickTime' },
  rpza: { friendlyName: 'Apple Video', codec: 'Road Pizza', category: 'Legacy/QuickTime' },
  'rle ': { friendlyName: 'QuickTime Animation', codec: 'RLE', category: 'Legacy/QuickTime' },
  'smc ': { friendlyName: 'Apple Graphics', codec: 'SMC', category: 'Legacy/QuickTime' },
  'raw ': { friendlyName: 'Uncompressed video', codec: 'Raw', category: 'Legacy/QuickTime' },
  '2vuy': { friendlyName: 'Uncompressed YUV 4:2:2', codec: 'YUV', category: 'Legacy/QuickTime' },
  yuv2: { friendlyName: 'YUV 4:2:2', codec: 'YUV', category: 'Legacy/QuickTime' },
  v210: { friendlyName: '10-bit uncompressed 4:2:2', codec: 'YUV', category: 'Legacy/QuickTime' },
  v410: { friendlyName: '10-bit uncompressed 4:4:4', codec: 'YUV', category: 'Legacy/QuickTime' },

  // --- DNx / Professional Codecs ---
  AVdn: { friendlyName: 'Avid DNxHD / DNxHR', codec: 'DNxHD', category: 'Professional' },
  CFHD: { friendlyName: 'CineForm HD', codec: 'CineForm', category: 'Professional' },
  icpf: { friendlyName: 'CineForm', codec: 'CineForm', category: 'Professional' },
  xdca: { friendlyName: 'XDCAM EX', codec: 'MPEG-2', category: 'Professional' },
  xd5b: { friendlyName: 'XDCAM HD422', codec: 'MPEG-2', category: 'Professional' },

  // --- Web / Streaming Oriented ---
  theo: { friendlyName: 'Theora', codec: 'Theora', category: 'Web/Streaming' },
  FLV1: { friendlyName: 'Flash Video', codec: 'Sorenson Spark', category: 'Web/Streaming' },
  H263: { friendlyName: 'H.263', codec: 'H.263', category: 'Web/Streaming' },
  DIVX: { friendlyName: 'DivX MPEG-4', codec: 'DivX', category: 'Web/Streaming' },
  XVID: { friendlyName: 'Xvid MPEG-4', codec: 'Xvid', category: 'Web/Streaming' },
  WMV1: { friendlyName: 'Windows Media Video 7', codec: 'WMV', category: 'Web/Streaming' },
  WMV2: { friendlyName: 'Windows Media Video 8', codec: 'WMV', category: 'Web/Streaming' },
  WMV3: { friendlyName: 'Windows Media Video 9', codec: 'WMV', category: 'Web/Streaming' },
}

/**
 * Normalizes input keys and looks up the codec information.
 * Exiftool occasionally outputs padded spaces or varying cases depending on the extraction context.
 */
export function getCodecInfo(compressorId: string | null | undefined): CodecInfo | null {
  if (!compressorId) {
    return null
  }

  // Check the exact match first
  if (COMPRESSOR_ID_MAP[compressorId]) {
    return COMPRESSOR_ID_MAP[compressorId]
  }

  // Fallback: Trim whitespace and evaluate case-insensitively
  const cleanId = compressorId.trim().toLowerCase()

  for (const [key, value] of Object.entries(COMPRESSOR_ID_MAP)) {
    if (key.trim().toLowerCase() === cleanId) {
      return value
    }
  }

  return null
}
