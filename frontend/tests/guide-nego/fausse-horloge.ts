/** Une horloge et une minuterie qu'on fait avancer à la main : les délais se prouvent sans attendre. */
export function fausseHorloge() {
  let maintenant = 0
  let suivant = 0
  const minuteurs = new Map<number, { echeance: number; fn: () => void }>()
  return {
    maintenant: () => maintenant,
    minuterie: {
      planifier(fn: () => void, ms: number) {
        minuteurs.set(++suivant, { echeance: maintenant + ms, fn })
        return suivant
      },
      annuler: (poignee: unknown) => void minuteurs.delete(poignee as number),
    },
    avancer(ms: number) {
      const cible = maintenant + ms
      for (;;) {
        const echu = [...minuteurs].filter(([, m]) => m.echeance <= cible).sort(([, a], [, b]) => a.echeance - b.echeance)[0]
        if (!echu) break
        minuteurs.delete(echu[0])
        maintenant = echu[1].echeance
        echu[1].fn()
      }
      maintenant = cible
    },
    enCours: () => minuteurs.size,
  }
}
