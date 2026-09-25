<script setup lang="ts">
import type { GlossaryEntry, KnowledgeSource } from '~/types/negotiation-savoir'
import { intituleDe } from '~/utils/guide-nego/lexique'

/**
 * Une entrée du lexique — maquette 06, écrans 04a et 04b. Elle se lit dans le savoir
 * gardé, sans réseau ; son adresse est sa désignation stable (FR-006). Public : l'état
 * « accès refusé » est sans objet. Le favori sans compte reste sur le téléphone.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const savoir = useGnSavoir()
const { noter } = useGnDerniersTermes()
const { retenir } = useGnOrigineDuLexique()
const { favoris, assurer: assurerLesFavoris, basculer } = useGnFavorisLexique()

const LEXIQUE = '/guide-nego/lexique'
const retour = ref(LEXIQUE)

onMounted(() => {
  retenir()
  const avant: unknown = window.history.state?.back
  if (typeof avant === 'string' && avant !== route.fullPath) retour.value = avant
  void savoir.assurer()
  void assurerLesFavoris()
})

const slug = computed(() => String(route.params.slug ?? ''))
const entree = computed<GlossaryEntry | null>(() => savoir.termeDe(slug.value))
const etat = computed(() => savoir.etat.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)

watch(
  () => entree.value?.id ?? null,
  (id) => {
    if (id && import.meta.client) noter(id)
  },
  { immediate: true },
)

const famille = computed(() => savoir.familles.value.find((f) => f.code === entree.value?.family_code)?.label)
const lies = computed(() =>
  (entree.value?.related_ids ?? []).flatMap((id) => savoir.lexique.value.find((e) => e.id === id) ?? []),
)
const estFavori = computed(() => !!entree.value && favoris.value.has(entree.value.id))

function libelleDeSource(source: KnowledgeSource): string {
  const titre = source.document_title ?? source.external_title ?? ''
  const { page_from: de, page_to: a } = source
  const pages = de === undefined ? '' : a !== undefined && a !== de ? t('guide-nego.lexique-entree.pages', { de, a }) : t('guide-nego.lexique-entree.page', { page: de })
  return [titre, source.section_label, pages].filter(Boolean).join(', ')
}

const versLaSource = (source: KnowledgeSource): string | null =>
  source.document_id ? `/guide-nego/ressources/documents/${source.document_id}` : (source.external_url ?? null)

const message = ref<{ texte: string; rang: number } | null>(null)
const annoncer = (texte: string) => (message.value = { texte, rang: (message.value?.rang ?? 0) + 1 })

async function partager(): Promise<void> {
  const e = entree.value
  if (!e) return
  const adresse = `${window.location.origin}${window.location.pathname}`
  if (typeof navigator.share === 'function') {
    try {
      await navigator.share({ title: `${intituleDe(e)} — ${e.translation}`, url: adresse })
      return
    } catch (erreur) {
      if (erreur instanceof DOMException && erreur.name === 'AbortError') return
    }
  }
  try {
    await navigator.clipboard.writeText(adresse)
    annoncer(t('guide-nego.lexique-entree.partage.copie'))
  } catch {
    annoncer(t('guide-nego.lexique-entree.partage.echec', { adresse }))
  }
}

useHead({ title: computed(() => entree.value?.term ?? t('guide-nego.lexique-entree.titre')) })
</script>

<template>
  <GnEcran
    :titre="entree?.term ?? t('guide-nego.lexique-entree.titre')"
    :sous-titre="entree?.translation"
    :surtitre="famille"
    :terme="!!entree"
    :retour="retour"
    :onglets="false"
    lexique-ouvert
  >
    <GnChargement v-if="chargement" forme="squelette" :lignes="4" :libelle="t('guide-nego.lexique-entree.chargement')" />

    <GnEtatErreur
      v-else-if="jamaisLu"
      :titre="t('guide-nego.lexique-entree.erreur.titre')"
      :texte="t('guide-nego.lexique-entree.erreur.texte')"
    />

    <GnEtatVide
      v-else-if="!entree"
      picto="text-size"
      :titre="t('guide-nego.lexique-entree.absent.titre')"
      :texte="t('guide-nego.lexique-entree.absent.texte')"
      :sortie="t('guide-nego.lexique-entree.absent.sortie')"
      :sortie-vers="LEXIQUE"
    />

    <article v-else class="gn-terme">
      <div class="gn-terme__corps">
        <p class="gn-terme__definition">{{ entree.definition }}</p>

        <section v-if="entree.heard_in_room">
          <GnEnteteGroupe :titre="t('guide-nego.lexique-entree.entendu')" />
          <blockquote class="gn-terme__entendu" lang="en">« {{ entree.heard_in_room }} »</blockquote>
        </section>

        <dl v-if="entree.acronym || entree.sources.length" class="gn-terme__cles">
          <div v-if="entree.acronym" class="gn-terme__cle">
            <dt>{{ t('guide-nego.lexique-entree.sigle') }}</dt>
            <dd lang="en"><strong>{{ entree.acronym }}</strong> — <i>{{ entree.term }}</i></dd>
          </div>
          <div v-if="entree.sources.length" class="gn-terme__cle">
            <dt>{{ t('guide-nego.lexique-entree.sources') }}</dt>
            <dd>
              <template v-for="(source, i) in entree.sources" :key="i">
                <span v-if="i" aria-hidden="true"> · </span>
                <a
                  v-if="versLaSource(source)?.startsWith('http')"
                  :href="versLaSource(source) ?? undefined"
                  target="_blank"
                  rel="noopener noreferrer"
                >{{ libelleDeSource(source) }}</a>
                <NuxtLink v-else-if="versLaSource(source)" :to="versLaSource(source) ?? undefined">{{ libelleDeSource(source) }}</NuxtLink>
                <span v-else>{{ libelleDeSource(source) }}</span>
              </template>
            </dd>
          </div>
        </dl>

        <section v-if="lies.length">
          <GnEnteteGroupe :titre="t('guide-nego.lexique-entree.lies')" />
          <ul role="list" class="gn-terme__lies">
            <li v-for="lie in lies" :key="lie.id">
              <GnPilule :vers="`${LEXIQUE}/${lie.slug}`"><i lang="en">{{ intituleDe(lie) }}</i></GnPilule>
            </li>
          </ul>
        </section>
      </div>

      <div class="gn-terme__actions">
        <GnBouton variante="secondaire" largeur="demie" picto="star" :actif="estFavori" @clic="basculer(entree.id)">
          {{ t('guide-nego.lexique-entree.favori') }}
        </GnBouton>
        <GnBouton variante="secondaire" largeur="demie" picto="share" @clic="partager">
          {{ t('guide-nego.lexique-entree.partager') }}
        </GnBouton>
      </div>
    </article>

    <GnMessageEphemere v-if="message" :key="message.rang" :texte="message.texte" @fini="message = null" />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-terme) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-terme {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-terme__corps {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-terme__definition {
  padding-top: var(--gn-espace-16);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-terme__entendu {
  margin: var(--gn-espace-12) 0 0;
  padding: var(--gn-espace-4) 0 var(--gn-espace-4) var(--gn-espace-12);
  border-inline-start: var(--gn-filet-3) solid var(--gn-filet-fort);
  font-style: italic;
}

[data-app="guide-nego"] .gn-terme__cles {
  margin-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-terme__cle {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-16);
  padding-block: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-terme__cle dt {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-terme__cle dd {
  margin: 0;
  text-align: end;
  font-weight: var(--gn-graisse-demi-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-terme__cle a {
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-terme__lies {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-terme__actions {
  display: flex;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-espace-16);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}
</style>
