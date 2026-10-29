const ADMIN_SIDEBAR_COOKIE = 'epavillon_admin_sidebar'
const ONE_YEAR = 60 * 60 * 24 * 365

/**
 * Barre latérale du back-office repliée en rail d'icônes, sur grand écran.
 *
 * Cookie plutôt que `localStorage`, pour la même raison que le thème : le
 * serveur rend la colonne à la bonne largeur dès le premier affichage. Lu après
 * hydratation, le choix ferait sauter tout le contenu de 190 px à chaque page.
 *
 * Un écran peut replier la barre pour gagner de la place sans toucher à la
 * préférence (`persist: false`) : elle reprend au rechargement.
 */
export function useAdminSidebar() {
  const cookie = useCookie<'collapsed' | null>(ADMIN_SIDEBAR_COOKIE, {
    default: () => null,
    maxAge: ONE_YEAR,
    sameSite: 'lax',
    path: '/',
  })
  const collapsed = useState('admin-sidebar-collapsed', () => cookie.value === 'collapsed')

  function setCollapsed(value: boolean, options: { persist?: boolean } = {}): void {
    collapsed.value = value
    if (options.persist ?? true) cookie.value = value ? 'collapsed' : null
  }

  return {
    isCollapsed: readonly(collapsed),
    setCollapsed,
    collapse: (options?: { persist?: boolean }) => setCollapsed(true, options),
    expand: (options?: { persist?: boolean }) => setCollapsed(false, options),
    toggle: () => setCollapsed(!collapsed.value),
  }
}
