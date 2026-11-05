import { ouvrir, BASE } from './session.mjs'
import { propre, plan } from './outils.mjs'
const { browser, page } = await ouvrir()
const allerA = async (url) => { await page.goto(url, { waitUntil: 'networkidle' }); await propre(page); await page.waitForTimeout(1200) }
await allerA(`${BASE}/mon-organisation`)
const action = page.locator('a', { hasText: 'Image de couverture à ajouter' }).first()
await plan(page, 'a1-espace', { action, afaire: page.getByRole('heading', { name: 'À faire' }) })
await action.click(); await page.waitForLoadState('networkidle'); await propre(page); await page.waitForTimeout(2500)
const deposer = page.getByRole('button', { name: 'Déposer une image' })
const modele = page.getByRole('link', { name: 'Créer ou choisir un modèle' })
await plan(page, 'a2-dossier', { section: page.locator('#couverture'), deposer, modele })
const dossierUrl = page.url().split('#')[0]
await modele.click(); await page.waitForLoadState('networkidle'); await propre(page); await page.waitForTimeout(2500)
const tpl = (n) => page.locator('button', { has: page.getByText(n, { exact: true }) }).first()
const apercu = page.getByRole('region', { name: "L'image et ses formats" })
await plan(page, 'a3-atelier', { onglets: page.getByRole('button', { name: 'Modèles' }), liste: tpl('Texte et photo'), apercu, enregistrer: page.getByRole('button', { name: 'Enregistrer la couverture' }) })
for (const [i, n] of ['Texte et photo', 'Éphéméride', 'Revue', 'Diagonale', 'Duotone'].entries()) {
  await tpl(n).scrollIntoViewIfNeeded(); await tpl(n).click(); await page.waitForTimeout(900)
  await plan(page, `a4-modele-${i}`, { bouton: tpl(n), apercu })
}
await browser.close()
