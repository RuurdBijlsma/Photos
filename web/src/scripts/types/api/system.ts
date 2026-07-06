export interface DiskInfo {
  diskAvailable: number
  diskUsed: number
  diskTotal: number
}

export interface DiskStats {
  appDataDrive: DiskInfo
  mediaDrive: DiskInfo
  areSameDrive: boolean
}

export interface SystemStats {
  hasClusteredPeople: boolean
  hasClusteredPhotos: boolean
  allowFileModifications: boolean
  allowFileDeletion: boolean
  disk: DiskStats
  isIngesting: boolean
}
