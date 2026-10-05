<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import { intituleDe } from '~/utils/guide-nego/lexique'

/**
 * Le lexique, maquette Nuit 03 : champ, terme trouvé en grand, derniers consultés.
 * « Aa » l'ouvre de partout, champ actif ; le retour ramène à l'écran d'origine. Tout se lit sur le téléphone : aucune
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
const premier = computed<GlossaryEntry | null>(() => resultats.value.trouves[0]?.valeur ?? null)
const autres = computed(() => resultats.value.trouves.slice(1).map((r) => r.valeur))
// Le terme montré en grand ne se répète pas dessous.
const consultes = computed(() =>
  derniers.value.flatMap((id) => (id === premier.value?.id ? [] : (entrees.value.find((e) => e.id === id) ?? []))),
)

const vers = (e: GlossaryEntry) => `/guide-nego/lexique/${e.slug}`

const proposition = ref(false)

function ouvrirLePremier(): void {
  const premier = resultats.value.trouves[0]?.valeur
  if (premier) void navigateTo(vers(premier))
}

useHead({ title: t('guide-nego.lexique.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.lexique.titre')" :retour="origine" lexique-ouvert>
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
        <nav class="gn-lexique__sorties">
          <NuxtLink to="/guide-nego/lexique/liste" class="gn-lexique__sortie">
            <GnPicto nom="toc" :taille="20" />{{ t('guide-nego.lexique.parcourir') }}
          </NuxtLink>
          <NuxtLink to="/guide-nego/lexique/favoris" class="gn-lexique__sortie">
            <GnPicto nom="star" :taille="20" />{{ t('guide-nego.lexique.favoris') }}
          </NuxtLink>
        </nav>
      </template>

      <template v-else-if="premier">
        <p v-if="suggestion" class="gn-lexique__peut-etre">
          {{ t('guide-nego.lexique.peut-etre') }}
          <NuxtLink :to="vers(suggestion)" class="gn-lexique__suggestion" lang="en">{{ intituleDe(suggestion) }}</NuxtLink>
        </p>
        <div aria-live="polite">
          <GnCarteTerme :key="premier.id" :entree="premier" />
        </div>
        <p v-if="resultats.aussiDansLesTraductions" class="gn-lexique__compte">{{ t('guide-nego.lexique.traductions') }}</p>
        <section v-if="autres.length" class="gn-lexique__section">
          <h2 class="gn-lexique__titre-section">{{ t('guide-nego.lexique.autres') }}</h2>
          <ul role="list">
            <li v-for="e in autres" :key="e.id">
              <GnLigneTerme :entree="e" :vers="vers(e)" :surligne="cherche" />
            </li>
          </ul>
        </section>
      </template>

      <template v-else>
        <p class="gn-lexique__compte" role="status">
          {{ t('guide-nego.lexique.sur', { count: 0, total: entrees.length }, 0) }}
        </p>
        <div class="gn-lexique__aucun">
          <p class="gn-lexique__glyphe" aria-hidden="true">Aa</p>
          <h2 class="gn-lexique__aucun-titre">{{ t('guide-nego.lexique.aucun.titre', { terme: cherche }) }}</h2>
          <p class="gn-lexique__aucun-texte">{{ t('guide-nego.lexique.aucun.texte') }}</p>
          <GnBouton picto="plus" @clic="proposition = true">
            {{ t('guide-nego.lexique.aucun.proposer', { terme: cherche }) }}
          </GnBouton>
          <NuxtLink to="/guide-nego/lexique/liste" class="gn-lexique__parcourir">{{ t('guide-nego.lexique.parcourir') }}</NuxtLink>
        </div>
      </template>

      <section v-if="!chargement && !jamaisLu && !vide && consultes.length" class="gn-lexique__section">
        <h2 class="gn-lexique__titre-section">{{ t('guide-nego.lexique.derniers') }}</h2>
        <ul role="list">
          <li v-for="e in consultes" :key="e.id">
            <GnLigneTerme :entree="e" :vers="vers(e)" />
          </li>
        </ul>
      </section>
    </div>

    <GnFeuilleProposerTerme v-model="proposition" :terme="cherche" />
  </GnEcran>
</template>

<style>
/* La maquette 03 : 20 entre les blocs. */
[data-app="guide-nego"] .gn-lexique {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-lexique__compte,
[data-app="guide-nego"] .gn-lexique__peut-etre {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-lexique__compte {
  margin-bottom: calc(-1 * var(--gn-espace-12));
}

[data-app="guide-nego"] .gn-lexique__suggestion {
  color: var(--gn-texte);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-lexique__sorties {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

[data-app="guide-nego"] .gn-lexique__sortie {
  min-height: var(--gn-ligne-reglage);
  padding-inline: var(--gn-espace-16);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-radius: var(--gn-rayon-16);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-size: var(--gn-taille-16);
  font-weight: var(--gn-graisse-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-lexique__sortie .gn-picto {
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-lexique__sortie:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-lexique__section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

/* « Derniers consultés » : un titre de bloc en retrait, Sora 16 en texte secondaire. */
[data-app="guide-nego"] .gn-lexique__titre-section {
  margin-bottom: var(--gn-espace-4);
  color: var(--gn-texte-2);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-lexique__aucun {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--gn-espace-12);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
  text-align: center;
}

[data-app="guide-nego"] .gn-lexique__glyphe {
  color: var(--gn-accent);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-44);
  line-height: var(--gn-interligne-44);
  letter-spacing: var(--gn-approche-44);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-lexique__aucun-titre {
  color: var(--gn-titre);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-lexique__aucun-texte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-lexique__aucun .gn-bouton {
  align-self: stretch;
}

[data-app="guide-nego"] .gn-lexique__parcourir {
  min-height: var(--gn-bouton-rond);
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
