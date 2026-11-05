import { ouvrir, BASE } from './session.mjs'
import { propre, plan } from './outils.mjs'
const etat = process.argv[2] ?? 'modele'
const { browser, page } = await ouvrir()
const ID = '0198c1a0-0000-7040-8000-000000000001'
const aller = async (url) => { await page.goto(url, { waitUntil: 'networkidle' }); await propre(page); await page.waitForTimeout(2500) }
await aller(`${BASE}/mon-organisation`)
await plan(page, `${etat}-espace`, {})
await aller(`${BASE}/mon-organisation/dossiers/${ID}#couverture`)
await plan(page, `${etat}-dossier`, { section: page.locator('#couverture'), image: page.locator('#couverture img').first() })
const lien = page.getByRole('link', { name: "Ouvrir la page publique de l'activité" }).first()
const href = await lien.getAttribute('href'); console.log('public', href)
await aller(href.startsWith('http') ? href : BASE + href)
await plan(page, `${etat}-public`, { titre: page.locator('h1').first() })
await page.mouse.wheel(0, 420); await page.waitForTimeout(1200)
await plan(page, `${etat}-public-bas`, {})
await browser.close()
