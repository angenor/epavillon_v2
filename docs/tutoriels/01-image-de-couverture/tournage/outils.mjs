import { writeFileSync, readFileSync, existsSync } from 'node:fs'
export const CSS = `#nuxt-devtools-container,#vue-tracer-overlay,nuxt-devtools-inspect-panel{display:none!important}
body>button.fixed{display:none!important} *{caret-color:transparent!important}`
const reperes = existsSync('plans/reperes.json') ? JSON.parse(readFileSync('plans/reperes.json', 'utf8')) : {}
export async function propre(page) { await page.addStyleTag({ content: CSS }) }
export async function plan(page, nom, cibles = {}) {
  await page.waitForTimeout(600)
  await page.screenshot({ path: `plans/${nom}.png` })
  const boites = {}
  for (const [cle, loc] of Object.entries(cibles)) {
    const b = await loc.boundingBox()
    if (b) boites[cle] = { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) }
    else console.warn('introuvable', nom, cle)
  }
  reperes[nom] = boites
  writeFileSync('plans/reperes.json', JSON.stringify(reperes, null, 1))
  console.log('plan', nom, Object.keys(boites).join(','))
}
