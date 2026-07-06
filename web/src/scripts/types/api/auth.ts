// Maps to the UserRole enum
export type UserRole = 'admin' | 'user'

// Maps to the Tokens schema
export interface Tokens {
  accessToken: string
  refreshToken: string
  expiry: number
}

// Maps to the User schema
export interface BaseUser {
  id: number
  createdAt: string // ISO date string
  updatedAt: string // ISO date string
  name: string
  role: UserRole
  mediaFolder: string | null
  avatarId: string | null
}

export interface User extends BaseUser {
  email: string
}

// Maps to request body schemas for type safety in our service calls
export interface LoginUser {
  email: string
  password: string
}

export interface CreateUser {
  email: string
  name: string
  password: string
  token?: string
}

export interface RefreshTokenPayload {
  refreshToken: string
}

export interface UserInvite {
  token: string
  expires_at: Date
  created_at: Date
}
