import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normaliserLeCherche, normaliserPourReperer } from '../../app/utils/guide-nego/pdf/normaliser.ts'

// Chaque cas se lit des deux côtés : la chaîne de PDFium (forme lisible, index de
// recherche) et les éléments de la couche de texte de pdf.js, tels qu'il les rend.
function seRejoignent(pdfium: string, pdfjs: string[], attendu: string): void {
  assert.equal(normaliserLeCherche(pdfium), attendu, 'côté PDFium')
  assert.equal(normaliserPourReperer(pdfjs).texte, attendu, 'côté pdf.js')
}

test('césure : U+0002 de PDFium, trait d’union en fin d’élément de pdf.js, ou rien', () => {
  seRejoignent('capitaux supplémentaires au\u0002delà de', ['capitaux supplémentaires au-', 'delà de'], 'capitauxsupplementairesaudelade')
  seRejoignent('socio\u0002écologique', ['socio', 'écologique'], 'socioecologique')
  seRejoignent('de passer à une phase post-négociation axée', ['de passer à une phase post-', 'négociation axée'], 'depasseraunephasepostnegociationaxee')
  seRejoignent('Glasgow–Charm el-Cheikh', ['Glasgow', '–', 'Charm', 'el-', 'Cheikh'], 'glasgowcharmelcheikh')
})

test('ligatures : pdf.js les garde (la couche est lue sans normalisation), PDFium les décompose', () => {
  seRejoignent('afin de faire fléchir', ['aﬁn de faire ﬂéchir'], 'afindefaireflechir')
  seRejoignent('efficace, affluer, offre', ['eﬃcace, aﬄuer, oﬀre'], 'efficace,affluer,offre')
})

test('insécables et fines deviennent des blancs, et les blancs ne comptent pas', () => {
  seRejoignent('1,55 °C', ['1,55 °C'], '1,55°c')
  seRejoignent('et les « Autres', ['et les « Autres'], 'etlesautres')
})

test('apostrophes et guillemets', () => {
  seRejoignent('avant que l’ordre du', ["avant que l'ordre du"], "avantquel'ordredu")
  seRejoignent('Africa” “Responding', ['Africa', '”', '“Responding'], 'africaresponding')
  seRejoignent('le « Rulebook »', ['le "Rulebook"'], 'lerulebook')
})

test('les fragments se recollent, quel que soit le découpage de la ligne', () => {
  seRejoignent('de l’Article 6 lors de la CdP29 à Bakou', ['de l’Article 6 lors de la', ' ', 'CdP29 à Bakou'], "del'article6lorsdelacdp29abakou")
  seRejoignent("pour le FRLD. L'enjeu est la mobilisation", ['pour le FRLD', ". L'enjeu est la", 'mobilisation'], "pourlefrld.l'enjeuestlamobilisation")
})

test('accents et casse : comme la recherche de l’étape 1', () => {
  seRejoignent('ŒUVRE Écologique', ['œuvre écologique'], 'oeuvreecologique')
})

test('la table ramène chaque lettre à son élément et à son caractère d’origine', () => {
  const { texte, table } = normaliserPourReperer(['par paliers).', '▪', ' ', 'L’élargissement'])
  const i = texte.indexOf("l'e")
  assert.deepEqual(table[i], { element: 3, caractere: 0 })
  assert.deepEqual(table[i + 1], { element: 3, caractere: 1 }, 'l’apostrophe courbe')
  assert.deepEqual(table[i + 2], { element: 3, caractere: 2 }, 'le É, replié en e')
  const { table: t2 } = normaliserPourReperer(['a ﬁn'])
  assert.deepEqual([t2[1], t2[2]], [{ element: 0, caractere: 2 }, { element: 0, caractere: 2 }], 'les deux lettres de la ligature')
  assert.equal(t2.length, 4)
})

test('le second temps écarte puces, numéros de liste et appels de note seuls sur leur élément', () => {
  const avec = normaliserPourReperer(['progression (linéaire ou par paliers).', '▪', ' ', 'L’élargissement de'])
  const sans = normaliserPourReperer(['progression (linéaire ou par paliers).', '▪', ' ', 'L’élargissement de'], { sansMarques: true })
  assert.match(avec.texte, /paliers\)\.▪l'e/u)
  assert.match(sans.texte, /paliers\)\.l'elargissement/u)
  for (const marque of ['•', '◦', '■', 'o', '', '5.', '12', 'a)', ' 15 ']) {
    assert.equal(normaliserPourReperer(['avant', marque, 'après'], { sansMarques: true }).texte, 'avantapres', `« ${marque} » écarté`)
  }
})

test('le second temps garde un nombre qui fait partie d’une phrase', () => {
  const { texte } = normaliserPourReperer(['de 758 millions de', 'dollars, en 2025.'], { sansMarques: true })
  assert.equal(texte, 'de758millionsdedollars,en2025.')
  assert.equal(normaliserPourReperer(['Article 6.4'], { sansMarques: true }).texte, 'article6.4')
})

test('les éléments vides ou blancs ne produisent rien, et ne décalent pas la table', () => {
  const { texte, table } = normaliserPourReperer(['', ' ', 'ab'])
  assert.equal(texte, 'ab')
  assert.deepEqual(table[0], { element: 2, caractere: 0 })
})
