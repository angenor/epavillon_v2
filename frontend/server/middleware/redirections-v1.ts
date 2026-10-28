import table from '../utils/redirections-v1.json'
import { redirectionV1, type TableRedirectionsV1 } from '../utils/redirections-v1'

export default defineEventHandler((event) => {
  const { pathname, search } = getRequestURL(event)
  const cible = redirectionV1(pathname, search, table as TableRedirectionsV1)
  if (cible === null) return
  // Une journée seulement : une séance publiée plus tard change la cible, et le
  // navigateur garderait sinon l'ancienne indéfiniment.
  setResponseHeader(event, 'cache-control', 'public, max-age=86400')
  return sendRedirect(event, cible, 301)
})
