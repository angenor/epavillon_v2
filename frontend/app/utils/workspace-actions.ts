import type { WorkspaceAction } from '~/types/organization-workspace'

/**
 * Icône et gravité de chaque nature d'action, partagées par l'espace organisation
 * et le menu du compte : jaune pour ce qui demande attention, cyan pour ce qui
 * informe. Rien en rouge — ce sont des choses à faire, pas des échecs. L'ordre
 * des clés est celui de l'urgence.
 */
export const WORKSPACE_ACTION_PRESENTATION: Record<
  WorkspaceAction['kind'],
  { icon: string; tone: 'warning' | 'info' }
> = {
  changes_requested: { icon: 'warning', tone: 'warning' },
  draft_before_deadline: { icon: 'edit', tone: 'warning' },
  coorganization_to_confirm: { icon: 'building', tone: 'info' },
  membership_request: { icon: 'users', tone: 'info' },
  session_report_missing: { icon: 'document', tone: 'info' },
}

/** Le détail d'une action ; seules les corrections demandées se comptent. */
export function workspaceActionDetail(
  action: WorkspaceAction,
  t: (key: string, count?: number) => string,
): string {
  const key = `organization.workspace.actions.kind.${action.kind}.detail`
  return action.kind === 'changes_requested' ? t(key, action.count) : t(key)
}
