import { ouvrir, BASE } from './session.mjs'
import { propre, plan } from './outils.mjs'
const { browser, page } = await ouvrir()
await page.goto(`${BASE}/`, { waitUntil: 'networkidle' }); await propre(page); await page.waitForTimeout(2500)
const menu = page.getByRole('button', { name: /Menu du compte/ }).first()
await plan(page, 'm0-accueil', { menu })
await menu.click(); await page.waitForTimeout(900)
await plan(page, 'm1-menu', { menu })
console.log(JSON.stringify(await page.locator('a:visible').evaluateAll(as => as.filter(a => a.getBoundingClientRect().top > 80 && a.getBoundingClientRect().left > 900).map(a => [a.innerText.replace(/\s+/g,' ').trim(), a.getAttribute('href'), Math.round(a.getBoundingClientRect().left), Math.round(a.getBoundingClientRect().top), Math.round(a.getBoundingClientRect().width), Math.round(a.getBoundingClientRect().height)]).slice(0, 15))))
await browser.close()
