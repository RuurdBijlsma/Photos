import apiClient from '@/scripts/services/api.ts'
import { copyToClipboard, getThumbnailHeight, getVideoHeight } from '@/scripts/utils.ts'

export async function navigatorShare(id: string, isVideo: boolean, hasThumbnails?: boolean, filename?: string, usePanoViewer?: boolean) {
  if (!id) return

  const isPano = usePanoViewer ?? false
  const shareLetter = isVideo ? 'v' : isPano ? 'pano' : 'p'
  const shareUrl = `${window.location.origin}/share/${shareLetter}/${id}`

  try {
    if (!navigator.share) {
      throw new Error('Web Share API is not supported in this browser environment.')
    }

    filename = filename || (isVideo ? 'video.mp4' : 'photo.jpg')
     hasThumbnails = hasThumbnails ?? true
    const onDemand = !hasThumbnails

    let url = ''
    if (isVideo) {
      const baseUrl = apiClient.defaults.baseURL
      const vidHeight = getVideoHeight(1440)
      const path = onDemand
        ? `/photos/${id}/video`
        : `/hosted/thumbnails/${id}/${vidHeight}p.webm`
      url = new URL(path, baseUrl).href
    } else {
      const baseUrl = apiClient.defaults.baseURL
      const thumbHeight = getThumbnailHeight(1440)
      const path = onDemand
        ? `/photos/${id}/thumbnail?size=${thumbHeight}`
        : `/hosted/thumbnails/${id}/${thumbHeight}p.avif`
      url = new URL(path, baseUrl).href
    }

    if (!url) {
      throw new Error('Unable to resolve media source URL.')
    }

    // Download the resource directly as a Blob
    const response = await apiClient.get<Blob>(url, { responseType: 'blob' })
    const blob = response.data

    const fileType = blob.type || (isVideo ? 'video/webm' : 'image/avif')

    // Adjust file extension based on actual MIME type returned by the server to prevent target app errors
    let sharedFilename = filename
    if (fileType.includes('avif') && !sharedFilename.toLowerCase().endsWith('.avif')) {
      sharedFilename = sharedFilename.replace(/\.[^/.]+$/, '') + '.avif'
    } else if (fileType.includes('webm') && !sharedFilename.toLowerCase().endsWith('.webm')) {
      sharedFilename = sharedFilename.replace(/\.[^/.]+$/, '') + '.webm'
    } else if (
      fileType.includes('jpeg') &&
      !sharedFilename.toLowerCase().endsWith('.jpg') &&
      !sharedFilename.toLowerCase().endsWith('.jpeg')
    ) {
      sharedFilename = sharedFilename.replace(/\.[^/.]+$/, '') + '.jpg'
    }

    const file = new File([blob], sharedFilename, { type: fileType })

    // Validate and use file sharing if supported by browser capabilities
    if (navigator.canShare && navigator.canShare({ files: [file] })) {
      await navigator.share({
        files: [file],
        title:filename,
      })
    } else {
      // Fallback 1: Share standard Web Share parameters
      await navigator.share({
        title: filename,
        url: shareUrl,
      })
    }
  } catch (error: any) {
    // Graceful cancellation: Do not copy to clipboard if the user manually exited the share dialog
    if (error?.name === 'AbortError') {
      console.log('Sharing operation dismissed by the user.')
      return
    }

    console.warn(
      'Native share process failed or was unsupported. Attempting text-only fallback:',
      error,
    )

    // Fallback 2: Try sharing text & URL even if file sharing threw an error
    if (navigator.share) {
      try {
        await navigator.share({
          title: filename,
          url: shareUrl,
        })
        return
      } catch (innerError: any) {
        if (innerError?.name === 'AbortError') {
          return
        }
      }
    }

    // Fallback 3: Copy shareable link to clipboard and notify user
    await copyToClipboard(shareUrl)
  }
}
