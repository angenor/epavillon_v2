<script setup lang="ts">
import type { NetworkMeeting, OfficialSession } from '~/types/negotiation-sessions'
import type { EtatAffiche } from '~/utils/guide-nego/sessions'

/**
 * Une session officielle dans la liste d'un jour (07 · 4 septies). Toute la ligne ouvre
 * la fiche. Le titre anglais fait foi : il reste affiché sous la traduction, qui n'a
 * été relue par personne et le dit.
 *
 * Les heures se lisent dans le fuseau de la COP ; l'écran le nomme une fois au-dessus
 * de la liste, et la ligne le redit à qui l'écoute.
 *
 * Une réunion non annoncée (`reunion`) prend la même ligne, sans type, ni titre anglais,
 * ni accès : elle ne se présente jamais comme une session officielle (FR-018).
 */
const props = withDefaults(
  defineProps<{
    /** Une session officielle, ou bien une réunion non annoncée : l'une des deux. */
    session?: OfficialSession | null
    reunion?: NetworkMeeting | null
    etat?: EtatAffiche
    fuseau: string
    /** Le nom du lieu pour « heure d'Antalya » ; à défaut, celui du fuseau. */
    ville?: string | null
    /** Le libellé de la thématique, déjà résolu depuis la base ; nul si elle n'en a pas. */
    thematique?: string | null
    /** Une coordination d'un groupe que la personne a coché. */
    monGroupe?: boolean
    vers: string
    /** « Mon agenda » (09 · 3a) : titre seul, salle et thématique sur une ligne, ni type ni accès. */
    forme?: 'liste' | 'agenda'
    /** Les sessions suivies qui la chevauchent : une marque par session, qui la nomme. */
    chevauche?: readonly OfficialSession[]
    rappel?: boolean
    /** L'heure de validation du dernier encart affiché (« HH:MM ») : le repère « Signalé ». */
    signale?: string | null
  }>(),
  {
    session: null,
    reunion: null,
    etat: 'prevue',
    ville: null,
    thematique: null,
    monGroupe: false,
    forme: 'liste',
    chevauche: () => [],
    rappel: false,
    signale: null,
  },
)

const { t, locale } = useI18n()
const { time, timeRange } = useDateTime()
const { tr } = useI18nText()

const debutIso = computed(() => props.session?.start_at ?? props.reunion?.start_at ?? null)
const finIso = computed(() => props.session?.end_at ?? null)
const debut = computed(() => (debutIso.value ? time(debutIso.value, props.fuseau) : null))
const fin = computed(() => (finIso.value ? time(finIso.value, props.fuseau) : null))
const ancienneHeure = computed(() => {
  const avant = props.session?.previous
  if (props.etat !== 'deplacee' || !avant) return null
  const ancienne = time(avant.start_at, props.fuseau)
  return ancienne !== debut.value ? ancienne : null
})
const heuresEntendues = computed(() =>
  debutIso.value
    ? timeRange(debutIso.value, finIso.value, props.fuseau, props.ville ?? undefined)
    : t('gn-ligne-session.sans-heure'),
)

/** En anglais, la traduction française n'a rien à faire à l'écran. */
const traduit = computed(() => (locale.value === 'fr' ? (props.session?.title_fr ?? null) : null))
const titre = computed(() => props.reunion?.title ?? traduit.value ?? props.session?.title_en ?? '')

const type = computed(() => (props.session?.type ? tr(props.session.type.label) : null))

const rattachement = computed(() => {
  if (props.monGroupe) return t('gn-ligne-session.mon-groupe')
  if (props.session?.group) return tr(props.session.group.label)
  return props.thematique ?? t('gn-ligne-session.sans-thematique')
})

const salle = computed(() => props.session?.venue ?? props.reunion?.venue ?? null)
const nonAnnoncee = computed(() =>
  props.reunion
    ? t('gn-ligne-session.non-annoncee', { heure: time(props.reunion.validated_at, props.fuseau) })
    : null,
)
const signaleEntendu = computed(() =>
  props.signale ? t('gn-ligne-session.signale-entendu', { heure: props.signale }) : null,
)

const agenda = computed(() => props.forme === 'agenda')

const marquesDeChevauchement = computed(() =>
  props.chevauche.map((autre) => {
    const debutAutre = time(autre.start_at, props.fuseau)
    const heures = autre.end_at ? `${debutAutre}–${time(autre.end_at, props.fuseau)}` : debutAutre
    const titreAutre = (locale.value === 'fr' ? autre.title_fr : null) ?? autre.title_en
    return { id: autre.id, texte: t('gn-ligne-session.chevauche', { heures, titre: titreAutre }) }
  }),
)

const annulee = computed(() => !!props.session && props.etat === 'annulee')
const terminee = computed(() => !!props.session && props.etat === 'terminee')
</script>

<template>
  <NuxtLink
    :to="vers"
    class="gn-ligne-session"
    :class="{ 'gn-ligne-session--annulee': annulee, 'gn-ligne-session--terminee': terminee }"
  >
    <span class="gn-ligne-session__heures">
      <span class="gn-ligne-session__entendu">{{ heuresEntendues }}</span>
      <s v-if="ancienneHeure" class="gn-ligne-session__ancienne" aria-hidden="true">{{ ancienneHeure }}</s>
      <span
        v-if="debut"
        class="gn-ligne-session__debut"
        :class="{ 'gn-ligne-session__debut--en-cours': session && etat === 'en-cours' }"
        aria-hidden="true"
      >{{ debut }}</span>
      <span v-else class="gn-ligne-session__debut" aria-hidden="true">—</span>
      <span v-if="fin" class="gn-ligne-session__fin" aria-hidden="true">{{ fin }}</span>
    </span>

    <span v-if="reunion" class="gn-ligne-session__corps">
      <span class="gn-ligne-session__titre">{{ titre }}</span>
      <span v-if="agenda" class="gn-ligne-session__lieu">
        <span v-if="salle" class="gn-ligne-session__salle">{{ salle }}</span>
        <span v-if="thematique" class="gn-ligne-session__thematique">{{ thematique }}</span>
      </span>
      <template v-else>
        <span v-if="salle" class="gn-ligne-session__lieu">
          <span class="gn-ligne-session__salle">{{ salle }}</span>
        </span>
        <span v-if="thematique" class="gn-ligne-session__thematique">{{ thematique }}</span>
      </template>
      <GnMarqueEtat etat="non-annoncee" :libelle="nonAnnoncee ?? undefined" />
    </span>

    <span v-else-if="session" class="gn-ligne-session__corps">
      <span v-if="type && !agenda" class="gn-ligne-session__type">{{ type }}</span>
      <span class="gn-ligne-session__titre">{{ titre }}</span>
      <template v-if="traduit && !agenda">
        <span class="gn-ligne-session__anglais" lang="en">
          <abbr class="gn-ligne-session__en" :title="t('gn-ligne-session.anglais')">EN</abbr>
          {{ session.title_en }}
        </span>
        <span class="gn-ligne-session__traduction">
          <GnPicto nom="translate" :taille="18" />
          {{ t('gn-ligne-session.traduction') }}
        </span>
      </template>
      <span v-if="agenda" class="gn-ligne-session__lieu">
        <span v-if="session.venue" class="gn-ligne-session__salle">{{ session.venue }}</span>
        <span class="gn-ligne-session__thematique">{{ rattachement }}</span>
      </span>
      <template v-else>
        <span v-if="session.venue || session.open_access !== null" class="gn-ligne-session__lieu">
          <span v-if="session.venue" class="gn-ligne-session__salle">{{ session.venue }}</span>
          <GnMarqueEtat
            v-if="session.open_access !== null"
            :etat="session.open_access ? 'ouverte' : 'acces-limite'"
          />
        </span>
        <span class="gn-ligne-session__thematique">{{ rattachement }}</span>
      </template>
      <GnEtatSession v-if="!agenda || etat !== 'prevue'" :session="session" :etat="etat" :fuseau="fuseau" />
      <span v-if="signale" class="gn-ligne-session__signale">
        <GnPicto nom="diamond" :taille="20" />
        <span aria-hidden="true">{{ t('gn-ligne-session.signale') }}</span>
        <span class="gn-hors-ecran">{{ signaleEntendu }}</span>
      </span>
      <GnMarqueEtat v-for="m in marquesDeChevauchement" :key="m.id" etat="chevauche" :libelle="m.texte" />
      <span v-if="rappel" class="gn-ligne-session__rappel">
        <GnPicto nom="bell" :taille="16" />
        {{ t('gn-ligne-session.rappel') }}
      </span>
    </span>

    <GnPicto nom="chevron" :taille="24" class="gn-ligne-session__chevron" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-session {
  position: relative;
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-ligne-session:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-session:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-session__heures {
  flex: none;
  width: var(--gn-colonne-heure);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  font-variant-numeric: tabular-nums;
}

/* Lu, pas vu : l'heure complète avec son fuseau, pour qui écoute la ligne. */
[data-app="guide-nego"] .gn-ligne-session__entendu {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-ligne-session__debut {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

/* Le jaune dit « maintenant » : il couvre le début seul (option B1). */
[data-app="guide-nego"] .gn-ligne-session__debut--en-cours {
  padding-inline: var(--gn-espace-4);
  margin-inline-start: calc(-1 * var(--gn-espace-4));
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
}

[data-app="guide-nego"] .gn-ligne-session__fin,
[data-app="guide-nego"] .gn-ligne-session__ancienne {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-ligne-session__ancienne {
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-ligne-session__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-session__type {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-session__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-ligne-session__anglais,
[data-app="guide-nego"] .gn-ligne-session__traduction,
[data-app="guide-nego"] .gn-ligne-session__thematique {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-ligne-session__en {
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-ligne-session__traduction {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

[data-app="guide-nego"] .gn-ligne-session__lieu {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  column-gap: var(--gn-espace-12);
  padding-block-start: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-ligne-session__salle {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-ligne-session__rappel {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

/* Le repère d'un encart : losange, mot, couleur — jamais la couleur seule (FR-017). */
[data-app="guide-nego"] .gn-ligne-session__signale {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-reseau);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-session__chevron {
  color: var(--gn-picto-secondaire);
}

/* Terminée : tout passe en gris. */
[data-app="guide-nego"] .gn-ligne-session--terminee .gn-ligne-session__debut,
[data-app="guide-nego"] .gn-ligne-session--terminee .gn-ligne-session__titre,
[data-app="guide-nego"] .gn-ligne-session--terminee .gn-ligne-session__salle {
  color: var(--gn-texte-2);
}

/* Annulée : elle garde sa place, son titre est barré. */
[data-app="guide-nego"] .gn-ligne-session--annulee .gn-ligne-session__titre {
  color: var(--gn-texte-2);
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-ligne-session--annulee .gn-ligne-session__salle {
  color: var(--gn-texte-2);
}
</style>
