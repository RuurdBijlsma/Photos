import type { BaseUser } from './auth.ts'

export interface UserStats {
  photoCount: number
  videoCount: number
  albumCount: number
  sharedAlbumCount: number
}

export interface UserProfile extends BaseUser {
  stats: UserStats
  email: string | null
}

export interface SmallUser {
  id: number
  name: string
  avatarId: string
}
