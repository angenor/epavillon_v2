<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import { intituleDe } from '~/utils/guide-nego/lexique'

/**
 * Le lexique — maquette 06, écrans 01, 02 et 06. « Aa » l'ouvre de partout, champ
 * actif ; la croix ramène à l'écran d'origine. Tout se lit sur le téléphone : aucune
 * frappe ne questionne le réseau. Public : l'état « accès refusé » est sans objet.
 *
 * La saisie vit dans l'adresse (`?q=`) : revenir d'une entrée retrouve la recherche.
 * `?terme=` ouvre l'entrée qu'il désigne, sinon la recherche préremplie
 * (contracts/resolution-lexique.md).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const savoir = useGnSavoir()
const { origine, retenir } = useGnOrigineDuLexique()
const { derniers, relire: relireLesDerniers } = useGnDerniersTermes()

const lirePremier = (valeur: unknown): string => {
  const premier = Array.isArray(valeur) ? valeur[0] : valeur
  return typeof premier === 'string' ? premier : ''
}

const saisie = ref(lirePremier(route.query.q))
const resolution = ref(Boolean(lirePremier(route.query.terme)))
const zone = useTemplateRef<HTMLElement>('zone')

watch(saisie, (q) => {
  if (q !== lirePremier(route.query.q)) void router.replace({ query: q.trim() ? { q } : {} })
})
watch(
  () => lirePremier(route.query.q),
  (q) => {
    if (q !== saisie.value) saisie.value = q
  },
)

onMounted(async () => {
  retenir()
  relireLesDerniers()
  await savoir.assurer()
  const terme = lirePremier(route.query.terme)
  if (terme) {
    const trouvee = savoir.resoudre(terme)
    if (trouvee) {
      await navigateTo(`/guide-nego/lexique/${trouvee.slug}`, { replace: true })
      return
    }
    saisie.value = terme
    await router.replace({ query: { q: terme } })
  }
  resolution.value = false
  await nextTick()
  zone.value?.querySelector('input')?.focus()
})

const etat = computed(() => savoir.etat.value)
const entrees = computed(() => savoir.lexique.value)
const chargement = computed(() => !etat.value.pret || resolution.value)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)
const vide = computed(() => !chargement.value && !jamaisLu.value && entrees.value.length === 0)

const cherche = computed(() => saisie.value.trim())
const resultats = computed(() => savoir.chercherDansLeLexique(cherche.value))
const suggestion = computed<GlossaryEntry | null>(() =>
  resultats.value.vousCherchiezPeutEtre ? (resultats.value.trouves[0]?.valeur ?? null) : null,
)
const consultes = computed(() =>
  derniers.value.flatMap((id) => entrees.value.find((e) => e.id === id) ?? []),
)

const vers = (e: GlossaryEntry) => `/guide-nego/lexique/${e.slug}`

function ouvrirLePremier(): void {
  const premier = resultats.value.trouves[0]?.valeur
  if (premier) void navigateTo(vers(premier))
}

useHead({ title: t('guide-nego.lexique.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.lexique.titre')" :retour="origine" fermer :onglets="false" lexique-ouvert>
    <div ref="zone" class="gn-lexique">
      <GnChampRecherche
        v-model="saisie"
        :libelle="t('guide-nego.lexique.champ')"
        :indication="t('guide-nego.lexique.indication')"
        :disabled="jamaisLu || vide || undefined"
        @keydown.enter="ouvrirLePremier"
      />

      <GnChargement v-if="chargement" forme="squelette" :lignes="3" :libelle="t('guide-nego.lexique.chargement')" />

      <GnEtatErreur
        v-else-if="jamaisLu"
        :titre="t('guide-nego.lexique.erreur.titre')"
        :texte="t('guide-nego.lexique.erreur.texte')"
      />

      <GnEtatVide
        v-else-if="vide"
        picto="text-size"
        :titre="t('guide-nego.lexique.vide.titre')"
        :texte="t('guide-nego.lexique.vide.texte')"
      />

      <template v-else-if="!cherche">
        <p class="gn-lexique__compte">{{ t('guide-nego.lexique.compte', { count: entrees.length }, entrees.length) }}</p>
        <section v-if="consultes.length">
          <GnEnteteGroupe :titre="t('guide-nego.lexique.derniers')" />
          <ul role="list">
            <li v-for="e in consultes" :key="e.id">
              <GnLigneTerme :entree="e" :vers="vers(e)" />
            </li>
          </ul>
        </section>
        <nav class="gn-lexique__sorties">
          <NuxtLink to="/guide-nego/lexique/liste" class="gn-lexique__sortie">
            <GnPicto nom="toc" :taille="24" />{{ t('guide-nego.lexique.parcourir') }}
          </NuxtLink>
          <NuxtLink to="/guide-nego/lexique/favoris" class="gn-lexique__sortie">
            <GnPicto nom="star" :taille="24" />{{ t('guide-nego.lexique.favoris') }}
          </NuxtLink>
        </nav>
      </template>

      <template v-else-if="resultats.trouves.length">
        <p v-if="suggestion" class="gn-lexique__peut-etre">
          {{ t('guide-nego.lexique.peut-etre') }}
          <NuxtLink :to="vers(suggestion)" class="gn-lexique__suggestion" lang="en">{{ intituleDe(suggestion) }}</NuxtLink>
        </p>
        <ul role="list" class="gn-lexique__resultats" aria-live="polite">
          <li v-for="r in resultats.trouves" :key="r.valeur.id">
            <GnLigneTerme :entree="r.valeur" :vers="vers(r.valeur)" extrait :surligne="cherche" />
          </li>
        </ul>
        <p class="gn-lexique__compte">
          {{ t('guide-nego.lexique.trouves', { count: resultats.trouves.length }, resultats.trouves.length) }}
          <template v-if="resultats.aussiDansLesTraductions">{{ t('guide-nego.lexique.traductions') }}</template>
        </p>
      </template>

      <template v-else>
        <p class="gn-lexique__compte gn-lexique__compte--filet" role="status">
          {{ t('guide-nego.lexique.sur', { count: 0, total: entrees.length }, 0) }}
        </p>
        <div class="gn-lexique__aucun">
          <p class="gn-lexique__glyphe" aria-hidden="true">Aa</p>
          <h2 class="gn-lexique__aucun-titre">{{ t('guide-nego.lexique.aucun.titre', { terme: cherche }) }}</h2>
          <p class="gn-lexique__aucun-texte">{{ t('guide-nego.lexique.aucun.texte') }}</p>
          <GnBouton picto="plus" desactive aria-describedby="gn-lexique-bientot">
            {{ t('guide-nego.lexique.aucun.proposer', { terme: cherche }) }}
          </GnBouton>
          <p id="gn-lexique-bientot" class="gn-lexique__bientot">{{ t('guide-nego.lexique.aucun.bientot') }}</p>
          <NuxtLink to="/guide-nego/lexique/liste" class="gn-lexique__parcourir">{{ t('guide-nego.lexique.parcourir') }}</NuxtLink>
        </div>
      </template>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-lexique {
  display: flex;
  flex-direction: column;
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-lexique > .gn-champ-recherche:has(input:focus) {
  border-color: var(--gn-filet-fort);
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: var(--gn-focus-decalage);
}

[data-app="guide-nego"] .gn-lexique__compte,
[data-app="guide-nego"] .gn-lexique__peut-etre {
  padding-block: var(--gn-espace-12);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-lexique__compte--filet,
[data-app="guide-nego"] .gn-lexique__peut-etre {
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-lexique__suggestion {
  color: var(--gn-titre);
  font-style: italic;
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-lexique__sorties {
  display: flex;
  flex-direction: column;
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-lexique__sortie {
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-lexique__aucun {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
  text-align: center;
}

[data-app="guide-nego"] .gn-lexique__glyphe {
  color: var(--gn-titre);
  font-size: var(--gn-taille-32);
  line-height: var(--gn-interligne-32);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-lexique__aucun-titre {
  color: var(--gn-titre);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-lexique__aucun-texte,
[data-app="guide-nego"] .gn-lexique__bientot {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-lexique__aucun .gn-bouton {
  align-self: stretch;
}

[data-app="guide-nego"] .gn-lexique__parcourir {
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
