export interface UpdatePersonRequest {
  name?: string | null
  faceThumbId?: string | null
}

export interface MergePersonRequest {
  targetPersonId: string
}
