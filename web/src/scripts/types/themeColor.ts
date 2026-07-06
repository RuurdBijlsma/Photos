export interface Theme {
  source_color: string
  contrast_level: number
  variant: string
  schemes: DynamicSchemeCollection
  custom_colors: CustomColor[]
}

export interface CustomColor {
  value: string
  name: string
  blend: boolean
}

export interface DynamicSchemeCollection {
  light: DynamicScheme
  dark: DynamicScheme
}

export interface Palette {
  hue: number
  chroma: number
  tones: PaletteTones
}

export interface HCT {
  hue: number
  chroma: number
  tone: number
}

export interface PaletteTones {
  '10': string
  '20': string
  '30': string
  '40': string
  '50': string
  '60': string
  '70': string
  '80': string
  '90': string
}

export interface DynamicScheme {
  source_color_hct: HCT
  variant: string
  is_dark: boolean
  contrast_level: number
  primary_palette: Palette
  secondary_palette: Palette
  tertiary_palette: Palette
  neutral_palette: Palette
  neutral_variant_palette: Palette
  error_palette: Palette
  background: string
  surface: string
  surface_dim: string
  surface_bright: string
  surface_container_lowest: string
  surface_container_low: string
  surface_container: string
  surface_container_high: string
  surface_container_highest: string
  on_surface: string
  surface_variant: string
  on_surface_variant: string
  inverse_surface: string
  inverse_on_surface: string
  outline: string
  outline_variant: string
  shadow: string
  scrim: string
  surface_tint: string
  primary: string
  on_primary: string
  primary_container: string
  on_primary_container: string
  inverse_primary: string
  secondary: string
  on_secondary: string
  secondary_container: string
  on_secondary_container: string
  tertiary: string
  on_tertiary: string
  tertiary_container: string
  on_tertiary_container: string
  error: string
  on_error: string
  error_container: string
  on_error_container: string
  primary_fixed: string
  primary_fixed_dim: string
  on_primary_fixed: string
  on_primary_fixed_variant: string
  secondary_fixed: string
  secondary_fixed_dim: string
  on_secondary_fixed: string
  on_secondary_fixed_variant: string
  tertiary_fixed: string
  tertiary_fixed_dim: string
  on_tertiary_fixed: string
  on_tertiary_fixed_variant: string
}
