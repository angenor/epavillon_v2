import { chromium } from 'playwright'
import { readFileSync } from 'node:fs'
export const BASE = 'http://localhost:3051'
export async function ouvrir() {
  const env = Object.fromEntries(readFileSync('/Users/mac/Documents/projets/IFDD/epavillon_v2/.env', 'utf8').split('\n').filter(l => l.includes('=') && !l.startsWith('#')).map(l => [l.slice(0, l.indexOf('=')), l.slice(l.indexOf('=') + 1).replace(/^"|"$/g, '')]))
  const browser = await chromium.launch()
  const context = await browser.newContext({ viewport: { width: 1440, height: 810 }, deviceScaleFactor: 2, colorScheme: 'light', locale: 'fr-FR' })
  const page = await context.newPage()
  await page.goto(`${BASE}/connexion`, { waitUntil: 'networkidle' })
  await page.getByLabel(/adresse|courriel/i).first().fill('a.sowfall@roac-afrique.org')
  await page.getByLabel(/mot de passe/i).first().fill(env.NUXT_PUBLIC_DEMO_PASSWORD)
  await page.getByRole('button', { name: /se connecter|connexion/i }).first().click()
  await page.waitForURL(u => !u.pathname.startsWith('/connexion'), { timeout: 20000 })
  return { browser, context, page }
}
