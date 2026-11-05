import { ouvrir, BASE } from './session.mjs'
import { propre, plan } from './outils.mjs'
import { writeFileSync } from 'node:fs'
const { browser, page } = await ouvrir()
const ID = '0198c1a0-0000-7040-8000-000000000001'
await page.addInitScript(() => { const o = URL.createObjectURL; window.__blobs = []; URL.createObjectURL = (b) => { if (b instanceof Blob) window.__blobs.push(b); return o.call(URL, b) } })
await page.goto(`${BASE}/mon-organisation/dossiers/${ID}/couverture?nouveau=1`, { waitUntil: 'networkidle' }); await propre(page); await page.waitForTimeout(2500)
const tpl = (n) => page.locator('button', { has: page.getByText(n, { exact: true }) }).first()
const apercu = page.getByRole('region', { name: "L'image et ses formats" })
let i = 0
for (const n of ['Texte et photo', 'Éphéméride', 'Diagonale']) {
  await tpl(n).click(); await page.waitForTimeout(900)
  await plan(page, `b${i++}-modele`, { bouton: tpl(n), apercu })
}
const cyan = page.getByLabel('Cyan', { exact: true }).first()
await cyan.click(); await page.waitForTimeout(900)
await plan(page, 'b3-couleur', { couleur: cyan, apercu })
const inter = page.getByLabel(/Intervenants/).first()
await inter.click(); await page.waitForTimeout(900)
await plan(page, 'b4-sans-intervenants', { option: inter, apercu })
await inter.click(); await page.waitForTimeout(900)
await plan(page, 'b5-avec-intervenants', { option: inter, apercu })
const textes = page.getByRole('button', { name: 'Textes' })
await textes.click(); await page.waitForTimeout(900)
await plan(page, 'b6-textes', { onglet: textes, titre: page.getByLabel("Titre de l'activité").first() })
const enreg = page.getByRole('button', { name: 'Enregistrer la couverture' })
await plan(page, 'b7-avant-enregistrer', { enregistrer: enreg })
await enreg.click(); await page.waitForTimeout(2500)
await plan(page, 'b8-enregistre', { enregistrer: enreg })
const images = await page.evaluate(async () => Promise.all(window.__blobs.filter(b => b.type.startsWith('image/')).map(async b => ({ type: b.type, size: b.size, data: await new Promise(r => { const f = new FileReader(); f.onload = () => r(String(f.result).split(',')[1]); f.readAsDataURL(b) }) }))))
images.forEach((im, k) => { writeFileSync(`plans/rendu-${k}.${im.type.split('/')[1]}`, Buffer.from(im.data, 'base64')); console.log('rendu', k, im.type, im.size) })
await browser.close()
