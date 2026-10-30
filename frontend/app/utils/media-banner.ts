import type { ComputedRef, InjectionKey } from 'vue'

/** Le bandeau porte-t-il une photo ? Ses pastilles n'ont droit au verre que sur un média. */
export const MEDIA_BANNER_ON_MEDIA: InjectionKey<ComputedRef<boolean>> = Symbol('media-banner-on-media')
