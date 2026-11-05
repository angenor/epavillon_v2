import { ouvrir, BASE } from './session.mjs'
import { propre, plan } from './outils.mjs'
const { browser, page } = await ouvrir()
const ID = '0198c1a0-0000-7040-8000-000000000001'
await page.goto(`${BASE}/mon-organisation/dossiers/${ID}#couverture`, { waitUntil: 'networkidle' }); await propre(page); await page.waitForTimeout(2500)
const deposer = page.getByRole('button', { name: 'Déposer une image' })
await plan(page, 'c0-dossier', { deposer, section: page.locator('#couverture') })
const [chooser] = await Promise.all([page.waitForEvent('filechooser'), deposer.click()])
await chooser.setFiles('port-littoral.jpg'); await page.waitForTimeout(2500)
const dialog = page.getByRole('dialog').first()
const desc = dialog.locator('textarea').first()
const televerser = dialog.getByRole('button', { name: 'Téléverser' })
await plan(page, 'c1-recadrage', { dialog, desc, televerser })
// Déplace le cadre de recadrage : glisser au centre de l'éditeur.
const zone = dialog.locator('canvas').first()
const zb = await zone.boundingBox()
if (zb) { await page.mouse.move(zb.x + zb.width / 2, zb.y + zb.height / 2); await page.mouse.down(); await page.mouse.move(zb.x + zb.width / 2 - 110, zb.y + zb.height / 2, { steps: 12 }); await page.mouse.up(); await page.waitForTimeout(600) }
await plan(page, 'c2-cadre', { dialog, zone, televerser })
await desc.click(); await desc.fill("Port et front de mer d'une ville côtière"); await page.waitForTimeout(500)
await plan(page, 'c3-description', { desc, televerser })
await televerser.click(); await page.waitForTimeout(2500)
await plan(page, 'c4-televerse', { section: page.locator('#couverture') })
await browser.close()
