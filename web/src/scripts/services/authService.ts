import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type { User, LoginUser, CreateUser, UserInvite } from '@/scripts/types/api/auth.ts'

const authService = {
  register(data: CreateUser): Promise<AxiosResponse<User>> {
    return apiClient.post<User>('/auth/register', data)
  },

  login(data: LoginUser): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/auth/login', data)
  },

  logout(): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/auth/logout')
  },

  refreshSession(): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/auth/refresh')
  },

  getMe(): Promise<AxiosResponse<User>> {
    return apiClient.get<User>('/auth/me')
  },

  generateInvite(userFolder: string): Promise<AxiosResponse<UserInvite>> {
    return apiClient.post<UserInvite>('/auth/generate-invite', { userFolder })
  },
}

export default authService
