<script setup lang="ts">
import { grouperParLettre } from '~/utils/guide-nego/lexique'

/**
 * La liste alphabétique — maquette 06, écran 03 : groupes par lettre et comptes, rail
 * A–Z, filtres par famille (dans l'adresse, retrouvés au retour d'une entrée). Taper
 * dans le champ rouvre la recherche. Public : l'état « accès refusé » est sans objet.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const savoir = useGnSavoir()
const { origine, retenir } = useGnOrigineDuLexique()

onMounted(() => {
  retenir()
  void savoir.assurer()
  window.addEventListener('scroll', suivreLaLettre, { passive: true })
})
onBeforeUnmount(() => window.removeEventListener('scroll', suivreLaLettre))

const etat = computed(() => savoir.etat.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)
const vide = computed(() => !chargement.value && !jamaisLu.value && savoir.lexique.value.length === 0)

const choisies = computed<string[]>(() => {
  const brut = route.query.famille
  return typeof brut === 'string' ? brut.split(',').filter(Boolean) : []
})

function basculerLaFamille(code: string): void {
  const suivantes = choisies.value.includes(code) ? choisies.value.filter((c) => c !== code) : [...choisies.value, code]
  void router.replace({ query: suivantes.length ? { famille: suivantes.join(',') } : {} })
}

const groupes = computed(() => {
  if (!choisies.value.length) return savoir.parLettre.value
  const familles = new Set(choisies.value)
  return grouperParLettre(savoir.lexique.value.filter((e) => familles.has(e.family_code)))
})
const presentes = computed(() => groupes.value.map((g) => g.lettre))
const total = computed(() => groupes.value.reduce((n, g) => n + g.entrees.length, 0))

const saisie = ref('')
watch(saisie, (q) => {
  if (q.trim()) void navigateTo({ path: '/guide-nego/lexique', query: { q } })
})

const courante = ref<string | null>(null)
let visee: string | null = null
const idDe = (lettre: string) => `gn-lettre-${lettre === '#' ? 'autres' : lettre}`

function suivreLaLettre(): void {
  let vue: string | null = presentes.value[0] ?? null
  for (const lettre of presentes.value) {
    const titre = document.getElementById(idDe(lettre))
    if (titre && titre.getBoundingClientRect().top <= 8) vue = lettre
  }
  // En bas de page, les dernières lettres ne montent plus en haut : celle qu'on a visée reste la bonne.
  const enBas = window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 2
  const titreVise = visee ? document.getElementById(idDe(visee)) : null
  courante.value = enBas && titreVise && titreVise.getBoundingClientRect().top < window.innerHeight ? visee : vue
}
watch(groupes, () => nextTick(suivreLaLettre))

const ORDRE = [...'ABCDEFGHIJKLMNOPQRSTUVWXYZ#']

/** Une lettre vide mène au groupe suivant : le doigt qui glisse ne tombe jamais dans le vide. */
function sauter(lettre: string): void {
  const rang = ORDRE.indexOf(lettre)
  const cible = presentes.value.find((l) => ORDRE.indexOf(l) >= rang) ?? presentes.value[presentes.value.length - 1]
  if (!cible) return
  visee = cible
  document.getElementById(idDe(cible))?.scrollIntoView({ block: 'start', behavior: 'instant' })
  courante.value = cible
}

useHead({ title: t('guide-nego.lexique-liste.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.lexique-liste.titre')" :retour="origine" fermer lexique-ouvert>
    <div class="gn-liste">
      <GnChampRecherche
        v-model="saisie"
        :libelle="t('guide-nego.lexique-liste.champ')"
        :indication="t('guide-nego.lexique-liste.indication')"
      />

      <GnChargement v-if="chargement" forme="squelette" :lignes="5" :libelle="t('guide-nego.lexique-liste.chargement')" />

      <GnEtatErreur
        v-else-if="jamaisLu"
        :titre="t('guide-nego.lexique-liste.erreur.titre')"
        :texte="t('guide-nego.lexique-liste.erreur.texte')"
      />

      <GnEtatVide
        v-else-if="vide"
        picto="text-size"
        :titre="t('guide-nego.lexique-liste.vide.titre')"
        :texte="t('guide-nego.lexique-liste.vide.texte')"
      />

      <template v-else>
        <div class="gn-liste__familles" role="group" :aria-label="t('guide-nego.lexique-liste.familles')">
          <GnPilule
            v-for="f in savoir.familles.value"
            :key="f.code"
            :choisie="choisies.includes(f.code)"
            @clic="basculerLaFamille(f.code)"
          >
            {{ f.label }}
          </GnPilule>
        </div>
        <p class="gn-liste__compte" role="status">{{ t('guide-nego.lexique-liste.compte', { count: total }, total) }}</p>

        <div class="gn-liste__corps">
          <div class="gn-liste__groupes">
            <p v-if="!groupes.length" class="gn-liste__aucune">{{ t('guide-nego.lexique-liste.aucune') }}</p>
            <section v-for="g in groupes" :key="g.lettre" :aria-labelledby="idDe(g.lettre)">
              <GnEnteteGroupe :id="idDe(g.lettre)" :titre="g.lettre" :compteur="g.entrees.length" class="gn-liste__lettre" />
              <ul role="list">
                <li v-for="e in g.entrees" :key="e.id">
                  <GnLigneTerme :entree="e" :vers="`/guide-nego/lexique/${e.slug}`" />
                </li>
              </ul>
            </section>
          </div>
          <GnRailAlphabet v-if="groupes.length" :presentes="presentes" :courante="courante" class="gn-liste__rail" @choisir="sauter" />
        </div>
      </template>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-liste {
  display: flex;
  flex-direction: column;
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-liste__familles {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-liste__compte,
[data-app="guide-nego"] .gn-liste__aucune {
  padding-top: var(--gn-espace-12);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-liste__corps {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-liste__groupes {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-liste__lettre {
  scroll-margin-top: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-liste__rail {
  position: sticky;
  top: var(--gn-espace-8);
  margin-top: var(--gn-espace-16);
}
</style>
