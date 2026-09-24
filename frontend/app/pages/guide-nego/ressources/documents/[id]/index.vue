<script setup lang="ts">
import type { LibraryDocument } from '~/types/negotiation-documents'
import { ETAT_DE_LA_MARQUE, libelleDeCop, libelleDuTerme, marquesDeLigne } from '~/utils/guide-nego/documents'
import { tailleLisible } from '~/utils/guide-nego/place'

/**
 * La fiche d'un document — maquette 03, fiches 05a à 05f.
 *
 * Elle se lit **dans la bibliothèque gardée** : l'API ne sert pas de fiche par
 * document, et la fiche s'ouvre sans réseau avec ce qui a été lu. Un réservé sans
 * accès arrive déjà vidé de son résumé et de ses thématiques ; l'écran n'en cache
 * rien qu'il aurait reçu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { date } = useDateTime()
const route = useRoute()
const bibliothequeLue = useGnDocuments()
const { etat, rafraichir, bibliotheque, documents, telecharges, nouveaux, documentDe, marquerCommeVu } = bibliothequeLue
const copies = useGnCopies()
const { favoris, assurer: assurerLesFavoris, basculer } = useGnFavoris()
const session = useGnSession()
const connexion = useGnConnexion()
const acces = useGnAcces()

const BIBLIOTHEQUE = '/guide-nego/ressources/documents'
// Le retour rend la liste telle qu'on l'a quittée : l'application installée n'a pas de retour système.
const versLaBibliotheque = useGnRetourALaBibliotheque()
const fuseau = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'

const id = computed(() => String(route.params.id ?? ''))
const document = computed<LibraryDocument | null>(() => documentDe(id.value))
const vocabulaire = computed(() => bibliotheque.value?.vocabulary ?? null)

const verrouille = computed(() => !!document.value && document.value.restricted && !document.value.accessible)
const estLien = computed(() => document.value?.source === 'link')
const telecharge = computed(() => telecharges.value.has(id.value))
const copie = computed(() => copies.copies.value.find((c) => c.id === id.value) ?? null)
const progression = computed(() => copies.progressions.value[id.value] ?? null)
const enAttente = computed(() => copies.enAttente.value.includes(id.value))
const enLigne = computed(() => connexion.etat.value.enLigne)

// « Nouveau » se perd à l'ouverture : la fiche garde ce qu'il était quand elle s'est ouverte.
const nouveauALOuverture = ref(false)
type Issue = 'place' | 'panne' | 'refus'
const issue = ref<Issue | null>(null)

watch(
  () => document.value?.id ?? null,
  (ouvert) => {
    issue.value = null
    if (!ouvert) return
    nouveauALOuverture.value = nouveaux.value.has(ouvert)
    marquerCommeVu(ouvert)
  },
  { immediate: true },
)

// L'accès vient de s'ouvrir sous le verrou : la liste gardée le dit encore fermé.
watch(acces.ouvert, (ouvert) => {
  if (ouvert && verrouille.value) void rafraichir()
})

onMounted(async () => {
  void rafraichir()
  await session.assurer()
  assurerLesFavoris()
})

const libelleDuType = computed(() =>
  (document.value && vocabulaire.value ? libelleDuTerme(vocabulaire.value.types, document.value.type) : null) ?? '',
)

const sousTitre = computed(() => {
  const d = document.value
  if (!d) return undefined
  if (!libelleDuType.value) return d.publisher ?? undefined
  return d.publisher ? t('guide-nego.document.sous-titre', { type: libelleDuType.value, editeur: d.publisher }) : libelleDuType.value
})

const marques = computed(() => {
  const d = document.value
  if (!d) return []
  return marquesDeLigne(d, { telecharge: telecharge.value, nouveau: nouveauALOuverture.value, enLigne: enLigne.value }).map(
    (marque) => ({
      marque,
      etat: ETAT_DE_LA_MARQUE[marque],
      libelle:
        marque === 'lien-reseau' || marque === 'non-telecharge' ? t(`guide-nego.document.marque.${marque}`) : undefined,
    }),
  )
})

const taille = (octets: number | null) => (octets === null ? null : tailleLisible(octets, locale.value))

const pagesEtTaille = computed(() => {
  const d = document.value
  if (!d || d.source !== 'file') return null
  const morceaux = [
    d.page_count === null ? null : t('guide-nego.document.pages', { count: d.page_count }, d.page_count),
    taille(d.reading_bytes),
  ].filter((m): m is string => !!m)
  return morceaux.length ? morceaux.join(' · ') : null
})

const thematiques = computed(() => {
  const d = document.value
  if (!d || !vocabulaire.value) return []
  const termes = vocabulaire.value.themes
  // Une thématique désactivée n'est plus servie par le vocabulaire : elle ne se montre plus.
  return d.themes.flatMap((code) => {
    const libelle = libelleDuTerme(termes, code)
    return libelle ? [{ code, libelle }] : []
  })
})

function nomDeLangue(code: string): string {
  try {
    const nom = new Intl.DisplayNames([locale.value], { type: 'language' }).of(code) ?? code
    return nom.charAt(0).toLocaleUpperCase(locale.value) + nom.slice(1)
  } catch {
    return code
  }
}

interface Detail {
  cle: string
  valeur: string
  accent?: boolean
}

const details = computed<Detail[]>(() => {
  const d = document.value
  if (!d) return []
  const issu = d.issued_on ? date(d.issued_on, 'UTC') : ''
  const cop = d.cop ? vocabulaire.value?.cops.find((c) => c.id === d.cop) : undefined
  const lignes: Array<Detail | null> = estLien.value
    ? [
        issu ? { cle: 'date', valeur: issu } : null,
        d.publisher ? { cle: 'editeur', valeur: d.publisher } : null,
        d.link_host ? { cle: 'adresse', valeur: d.link_host, accent: true } : null,
        cop ? { cle: 'cop', valeur: libelleDeCop(cop) } : null,
        { cle: 'ajoute', valeur: date(d.published_at, fuseau) },
      ]
    : [
        d.version ? { cle: 'version', valeur: d.version } : null,
        issu ? { cle: 'date', valeur: issu } : null,
        d.publisher ? { cle: 'editeur', valeur: d.publisher } : null,
        { cle: 'langue', valeur: nomDeLangue(d.locale) },
        cop ? { cle: 'cop', valeur: libelleDeCop(cop) } : null,
      ]
  return lignes.filter((l): l is Detail => !!l && !!l.valeur)
})

const remplacant = computed(() => document.value?.superseded_by ?? null)
const remplacantReserveSansAcces = computed(() => {
  const suivant = remplacant.value ? documentDe(remplacant.value.id) : null
  return !!suivant && suivant.restricted && !suivant.accessible
})

type Principal = 'navigateur' | 'progression' | 'attente' | 'a-jour' | 'lire' | 'telecharger'

const principal = computed<Principal>(() => {
  if (estLien.value) return 'navigateur'
  if (progression.value) return 'progression'
  if (enAttente.value) return 'attente'
  if (remplacant.value) return 'a-jour'
  if (telecharge.value) return 'lire'
  return 'telecharger'
})

const versLaLecture = computed(() => `${BIBLIOTHEQUE}/${id.value}/lire`)
const versLeRemplacant = computed(() => (remplacant.value ? `${BIBLIOTHEQUE}/${remplacant.value.id}` : BIBLIOTHEQUE))

const avancement = computed(() => {
  const p = progression.value
  if (!p) return null
  const part = p.total && p.total > 0 ? Math.min(1, Math.max(0, p.recus / p.total)) : null
  const recus = tailleLisible(p.recus, locale.value)
  return {
    part,
    titre:
      part === null
        ? t('guide-nego.document.progression.sans-part')
        : t('guide-nego.document.progression.part', {
            part: new Intl.NumberFormat(locale.value, { style: 'percent', maximumFractionDigits: 0 }).format(part),
          }),
    octets:
      p.total === null
        ? t('guide-nego.document.progression.recus', { recus })
        : t('guide-nego.document.progression.octets', { recus, total: tailleLisible(p.total, locale.value) }),
  }
})

const libelleRetirer = computed(() => {
  const octets = taille(copie.value?.octets ?? null)
  return octets ? t('guide-nego.document.action.retirer', { taille: octets }) : t('guide-nego.document.action.retirer-sans-taille')
})

const texteDeLIssue = computed(() => {
  if (issue.value === 'place') {
    const besoin = taille(document.value?.reading_bytes ?? null)
    return besoin ? t('guide-nego.document.issue.place', { taille: besoin }) : t('guide-nego.document.issue.place-sans-taille')
  }
  return issue.value ? t(`guide-nego.document.issue.${issue.value}`) : ''
})

const telechargementDemande = ref(false)

async function telecharger(): Promise<void> {
  const d = document.value
  if (!d || telechargementDemande.value) return
  issue.value = null
  telechargementDemande.value = true
  try {
    const suite = await copies.telecharger(d)
    if (suite === 'place' || suite === 'panne' || suite === 'refus') issue.value = suite
    else if (suite === 'impossible') issue.value = 'refus'
    // Un refus dit peut-être un document dépublié ou devenu réservé : la liste le saura.
    if (suite === 'refus') void rafraichir()
  } finally {
    telechargementDemande.value = false
  }
}

const confirmationRetrait = ref(false)
const retraitEnCours = ref(false)

async function retirer(): Promise<void> {
  retraitEnCours.value = true
  try {
    await copies.retirer(id.value)
  } finally {
    retraitEnCours.value = false
  }
}

const feuilleFavori = ref(false)
const estFavori = computed(() => favoris.value.has(id.value))

function favori(): void {
  if (!session.connectee.value) {
    feuilleFavori.value = true
    return
  }
  void basculer(id.value)
}

const message = ref<{ texte: string; rang: number } | null>(null)
const annoncer = (texte: string) => (message.value = { texte, rang: (message.value?.rang ?? 0) + 1 })

// L'adresse de la fiche, jamais le contenu : un réservé reçu ainsi s'ouvre sur son verrou.
async function partager(): Promise<void> {
  const d = document.value
  if (!d) return
  // Le chemin affiché porte la base du site (`/v2/` en ligne), que `route.path` ignore.
  const adresse = `${window.location.origin}${window.location.pathname}`
  if (typeof navigator.share === 'function') {
    try {
      await navigator.share({ title: d.title, url: adresse })
      return
    } catch (erreur) {
      if (erreur instanceof DOMException && erreur.name === 'AbortError') return
    }
  }
  try {
    await navigator.clipboard.writeText(adresse)
    annoncer(t('guide-nego.document.partage.copie'))
  } catch {
    annoncer(t('guide-nego.document.partage.echec', { adresse }))
  }
}

// Hors connexion, un document qui n'est pas sur le téléphone ne se lit pas : le bandeau ne dit pas le contraire.
const ceQuiSeLit = computed(() =>
  document.value && (estLien.value || !telecharge.value) ? t('guide-nego.document.hors-connexion') : undefined,
)

const autresOuverts = computed(() => documents.value.filter((d) => !d.restricted).length)

useHead({ title: computed(() => document.value?.title ?? t('guide-nego.document.titre')) })
</script>

<template>
  <GnEcran
    :titre="document?.title ?? t('guide-nego.document.titre')"
    :sous-titre="sousTitre"
    :retour="versLaBibliotheque"
    :onglets="false"
    :ce-qui-se-lit="ceQuiSeLit"
  >
    <GnChargement v-if="!etat.pret" forme="squelette" :lignes="6" class="gn-fiche__attente" />

    <GnEtatErreur
      v-else-if="!bibliotheque"
      :titre="t('guide-nego.document.erreur.titre')"
      :texte="t('guide-nego.document.erreur.texte')"
      :sortie="t('guide-nego.document.erreur.sortie')"
      @sortie="rafraichir"
    />

    <GnEtatVide
      v-else-if="!document"
      picto="doc"
      :titre="t('guide-nego.document.vide.titre')"
      :texte="t(enLigne ? 'guide-nego.document.vide.texte' : 'guide-nego.document.vide.texte-hors-connexion')"
      :sortie="t('guide-nego.document.vide.sortie')"
      :sortie-vers="versLaBibliotheque"
    />

    <div v-else class="gn-fiche">
      <GnBandeauRemplace
        v-if="remplacant"
        class="gn-fiche__bandeau"
        :titre="remplacant.title"
        :publie-le="remplacant.published_at"
        :pages="remplacant.page_count"
        :telecharge="telecharges.has(remplacant.id)"
        :reserve-sans-acces="remplacantReserveSansAcces"
        :vers="versLeRemplacant"
      />

      <div class="gn-fiche__corps">
        <p v-if="marques.length || pagesEtTaille" class="gn-fiche__marques">
          <GnMarqueEtat v-for="m in marques" :key="m.marque" :etat="m.etat" :libelle="m.libelle" />
          <span v-if="pagesEtTaille" class="gn-fiche__taille">
            <GnPicto nom="doc" :taille="20" />
            {{ pagesEtTaille }}
          </span>
        </p>

        <GnVerrou
          v-if="verrouille"
          :propos="t('guide-nego.document.verrou.propos')"
          :reste-ouvert="t('guide-nego.document.verrou.reste-ouvert', { n: autresOuverts }, autresOuverts)"
          :retour-vers="versLaBibliotheque"
        />

        <template v-else>
          <GnEtiquette v-if="estLien" class="gn-fiche__lien" picto="wifi-off" :texte="t('guide-nego.document.lien')" />
          <GnEtiquette
            v-else-if="document.source === 'file' && !document.has_text"
            class="gn-fiche__lien"
            picto="doc"
            :texte="t('guide-nego.document.sans-texte')"
          />

          <p v-if="document.summary" class="gn-fiche__resume" :class="{ 'gn-fiche__resume--eteint': remplacant }">
            {{ document.summary }}
          </p>

          <section class="gn-fiche__section" :aria-labelledby="`${document.id}-thematiques`">
            <h2 :id="`${document.id}-thematiques`" class="gn-fiche__intertitre">
              <span>{{ t('guide-nego.document.thematiques.titre') }}</span>
              <span v-if="thematiques.length" class="gn-fiche__compte">{{ thematiques.length }}</span>
            </h2>
            <ul v-if="thematiques.length" class="gn-fiche__thematiques">
              <li v-for="theme in thematiques" :key="theme.code" class="gn-fiche__thematique">{{ theme.libelle }}</li>
            </ul>
            <p v-else class="gn-fiche__aucune">{{ t('guide-nego.document.thematiques.aucune') }}</p>
          </section>

          <section class="gn-fiche__section" :aria-labelledby="`${document.id}-details`">
            <h2 :id="`${document.id}-details`" class="gn-fiche__intertitre">
              <span>{{ t('guide-nego.document.details.titre') }}</span>
            </h2>
            <dl class="gn-fiche__details">
              <div v-for="detail in details" :key="detail.cle" class="gn-fiche__detail">
                <dt>{{ t(`guide-nego.document.details.${detail.cle}`) }}</dt>
                <dd :class="{ 'gn-fiche__detail--accent': detail.accent }">{{ detail.valeur }}</dd>
              </div>
            </dl>
          </section>

          <p v-if="telecharge && !progression" class="gn-fiche__garde">
            <GnPicto nom="check-circle" :taille="20" />
            <span class="gn-fiche__garde-texte">{{ t('guide-nego.document.telecharge') }}</span>
            <NuxtLink :to="versLaLecture" class="gn-fiche__garde-lien">{{ t('guide-nego.document.action.lire') }}</NuxtLink>
          </p>
        </template>
      </div>

      <div v-if="!verrouille" class="gn-fiche__actions">
        <div v-if="issue" class="gn-fiche__issue" role="alert">
          <GnPicto nom="warn" :taille="20" class="gn-fiche__issue-picto" />
          <span class="gn-fiche__issue-texte">{{ texteDeLIssue }}</span>
          <NuxtLink v-if="issue === 'place'" to="/guide-nego/ressources/mes-documents" class="gn-fiche__issue-sortie">
            {{ t('guide-nego.document.issue.place-sortie') }}
          </NuxtLink>
        </div>

        <GnBouton
          v-if="principal === 'navigateur' && document.external_url"
          :vers="document.external_url"
          target="_blank"
          rel="noopener noreferrer"
          picto="external"
        >
          {{ t('guide-nego.document.action.navigateur') }}
        </GnBouton>

        <div v-else-if="principal === 'progression' && avancement" class="gn-fiche__avancement" role="status">
          <span class="gn-fiche__avancement-ligne">
            <span class="gn-fiche__avancement-titre">
              <GnChargement :taille="20" decoratif />
              {{ avancement.titre }}
            </span>
            <span class="gn-fiche__avancement-octets">{{ avancement.octets }}</span>
          </span>
          <GnProgression :part="avancement.part" decoratif />
        </div>

        <p v-else-if="principal === 'attente'" class="gn-fiche__avancement gn-fiche__avancement-titre" role="status">
          <GnPicto nom="clock" :taille="20" />
          {{ t('guide-nego.document.en-attente') }}
        </p>

        <GnBouton v-else-if="principal === 'a-jour'" :vers="versLeRemplacant">
          {{ t('guide-nego.document.action.a-jour') }}
        </GnBouton>

        <GnBouton v-else-if="principal === 'lire'" :vers="versLaLecture">
          {{ t('guide-nego.document.action.lire') }}
        </GnBouton>

        <GnBouton v-else-if="principal === 'telecharger'" picto="download" :chargement="telechargementDemande" @clic="telecharger">
          {{ t(`guide-nego.document.action.${issue === 'panne' ? 'reessayer' : enLigne ? 'telecharger' : 'telecharger-au-retour'}`) }}
        </GnBouton>

        <div class="gn-fiche__paire">
          <GnBouton variante="secondaire" largeur="demie" picto="star" :actif="estFavori" @clic="favori">
            {{ t('guide-nego.document.action.favori') }}
          </GnBouton>
          <GnBouton variante="secondaire" largeur="demie" picto="share" @clic="partager">
            {{ t('guide-nego.document.action.partager') }}
          </GnBouton>
        </div>

        <GnBouton v-if="principal === 'progression'" variante="discret" @clic="copies.annuler(id)">
          {{ t('guide-nego.document.action.annuler') }}
        </GnBouton>
        <GnBouton v-else-if="principal === 'attente'" variante="discret" @clic="copies.annuler(id)">
          {{ t('guide-nego.document.action.annuler-demande') }}
        </GnBouton>
        <template v-else-if="!estLien">
          <GnBouton v-if="remplacant && telecharge" variante="discret" :vers="versLaLecture">
            {{ t('guide-nego.document.action.lire-cette-version') }}
          </GnBouton>
          <GnBouton
            v-else-if="remplacant"
            variante="discret"
            :chargement="telechargementDemande"
            @clic="telecharger"
          >
            {{ t('guide-nego.document.action.telecharger-quand-meme') }}
          </GnBouton>
          <GnBouton
            v-if="telecharge"
            variante="dangereux"
            :chargement="retraitEnCours"
            @clic="confirmationRetrait = true"
          >
            {{ libelleRetirer }}
          </GnBouton>
        </template>
      </div>
    </div>

    <GnConfirmation
      v-model="confirmationRetrait"
      :question="t('guide-nego.document.retirer.question')"
      :phrase="t('guide-nego.document.retirer.phrase')"
      :action="t('guide-nego.document.retirer.action')"
      dangereuse
      @confirmer="retirer"
    />

    <GnFeuilleBasse v-model="feuilleFavori" :titre="t('guide-nego.document.favori.titre')" :sous-titre="t('guide-nego.document.favori.texte')">
      <div class="gn-fiche__sorties-compte">
        <GnBouton vers="/guide-nego/compte" picto="user">{{ t('guide-nego.document.favori.creer') }}</GnBouton>
        <GnBouton variante="secondaire" vers="/guide-nego/connexion">{{ t('guide-nego.document.favori.connexion') }}</GnBouton>
      </div>
    </GnFeuilleBasse>

    <GnMessageEphemere v-if="message" :key="message.rang" :texte="message.texte" @fini="message = null" />
  </GnEcran>
</template>

<style>
/* Les actions se posent en bas, à portée de pouce, même quand la fiche est courte. */
[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-fiche) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-fiche {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-fiche__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-fiche__bandeau {
  margin-inline: calc(-1 * var(--gn-marge-ecran));
}

[data-app="guide-nego"] .gn-fiche__corps {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-fiche__marques {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--gn-espace-12);
  row-gap: var(--gn-espace-4);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-fiche__taille {
  display: inline-flex;
  align-items: center;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-fiche__lien {
  align-self: flex-start;
  margin-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-fiche__resume {
  margin-top: var(--gn-espace-12);
  max-width: var(--gn-mesure-lecture);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-fiche__resume--eteint {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-fiche__intertitre {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: var(--gn-espace-16) 0 6px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-fiche__compte {
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-fiche__thematiques {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
  padding-block: 10px;
  list-style: none;
}

[data-app="guide-nego"] .gn-fiche__thematique {
  min-height: var(--gn-filtre-hauteur);
  padding-inline: 14px;
  display: inline-flex;
  align-items: center;
  border: var(--gn-filet-2) solid var(--gn-filet);
  border-radius: var(--gn-rayon-24);
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-fiche__aucune {
  padding-block: 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-fiche__detail {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--gn-espace-12);
  padding-block: 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-fiche__detail:last-child {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-fiche__detail dt {
  flex: none;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-fiche__detail dd {
  min-width: 0;
  font-weight: var(--gn-graisse-demi-gras);
  text-align: end;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-fiche__detail--accent {
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-fiche__garde {
  min-height: var(--gn-cible);
  margin-top: var(--gn-espace-8);
  padding-inline: var(--gn-espace-16);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-fiche__garde-texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-fiche__garde-lien {
  flex: none;
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  color: var(--gn-action-sur-titre);
  font-weight: var(--gn-graisse-gras);
  text-decoration-line: var(--gn-action-sur-titre-trait);
  text-underline-offset: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-fiche__actions {
  position: sticky;
  bottom: 0;
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding: var(--gn-espace-16) var(--gn-marge-ecran) calc(var(--gn-espace-16) + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  background: var(--gn-fond);
}

[data-app="guide-nego"] .gn-fiche__paire {
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-fiche__issue {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-8) var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-fiche__issue-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-fiche__issue-texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-fiche__issue-sortie {
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-fiche__avancement {
  min-height: var(--gn-cible);
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 6px;
}

[data-app="guide-nego"] .gn-fiche__avancement-ligne {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  column-gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-fiche__avancement-titre {
  display: flex;
  flex-direction: row;
  justify-content: flex-start;
  align-items: center;
  gap: var(--gn-espace-8);
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-fiche__avancement-octets {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-fiche__sorties-compte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}
</style>
