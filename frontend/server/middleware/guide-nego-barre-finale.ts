import { avecBarreFinale, sansBarreFinale } from '../../app/utils/guide-nego/barre-finale'

export default defineEventHandler((event) => {
  const { pathname, search } = getRequestURL(event)
  if (!sansBarreFinale(pathname, useRuntimeConfig().app.baseURL)) return
  // Gardée par le navigateur, la redirection ramène sous la portée une adresse ouverte sans réseau.
  setResponseHeader(event, 'cache-control', 'public, max-age=31536000')
  return sendRedirect(event, avecBarreFinale(pathname, search), 301)
})
