/**
 * L'adresse qu'un navigateur ouvre pour un chemin d'API : un `<img>`, un lien de
 * téléchargement. Ce qui n'est pas du JSON ne passe pas par `useApi()`.
 *
 * Sans API branchée, `null` : le jeu d'exemple ne sert ni image ni PDF.
 */
export function useApiUrl(): (chemin: string) => string | null {
  const base = String(useRuntimeConfig().public.apiBase ?? '')
  return (chemin) => (base ? `${base.replace(/\/$/, '')}${chemin}` : null)
}
