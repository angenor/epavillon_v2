<script setup lang="ts">
import { reunionVisible } from '~/utils/guide-nego/signalements'

/**
 * La fiche d'une réunion non annoncée (07, ligne « Non annoncée » ; 08 pour la forme) :
 * quoi, où, quand, thématique. **Aucune section « Source officielle »** : elle vient du
 * réseau, validée par l'IFDD, et ne se présente jamais comme une session officielle.
 * Elle reste lisible quand la source est coupée (FR-022), et tombe à la fin de son jour.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { dayLong, time, zoneLabel, zoneOffsetShort } = useDateTime()
const route = useRoute()
const connexion = useGnConnexion()
const compte = useGnSession()
const thematiques = useGnThematiques()
const agenda = useGnAgenda()
const lecture = useGnSessions()
const { affichage, etat, sansEdition, edition } = lecture

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.negociations-reseau.${cle}`, params)

const id = computed(() => String(route.params.id ?? ''))
const retour = computed(() =>
  route.query.depuis === 'agenda' ? '/guide-nego/negociations/agenda' : '/guide-nego/negociations',
)

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    await lecture.rafraichir()
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void relire()
  await compte.assurer()
  agenda.assurer()
  await thematiques.assurerLeVocabulaire()
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const lue = computed(() => affichage.value.etat !== 'inconnu')
const attente = computed(() => !etat.value.pret && !tentee.value)
const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const ville = computed(() => lecture.ville.value ?? edition.value?.city ?? undefined)

const reunion = computed(() => {
  const r = lecture.reunion(id.value)
  return r && reunionVisible(r, maintenant.value, fuseau.value) ? r : null
})

const marque = computed(() =>
  reunion.value ? t('gn-ligne-session.non-annoncee', { heure: time(reunion.value.validated_at, fuseau.value) }) : '',
)

const quand = computed(() => {
  const r = reunion.value
  if (!r) return null
  const precision = k('heure-precision', {
    jour: dayLong(r.start_at ?? `${r.day}T12:00:00Z`, r.start_at ? fuseau.value : 'UTC'),
    zone: zoneLabel(fuseau.value, ville.value),
    decalage: zoneOffsetShort(fuseau.value, r.start_at ?? `${r.day}T12:00:00Z`),
  })
  return { valeur: r.start_at ? time(r.start_at, fuseau.value) : k('sans-heure'), precision }
})

const thematique = computed(() => {
  const code = reunion.value?.theme
  return (code ? thematiques.nomDe(code) : null) ?? k('sans-thematique')
})

// --- Agenda ------------------------------------------------------------------

const dansLAgenda = computed(() => agenda.reunionDansLAgenda(id.value))
const feuilleCompte = ref(false)
const message = ref<{ texte: string; action?: string; agir?: () => void; rang: number } | null>(null)
const annoncer = (texte: string, action?: string, agir?: () => void) =>
  (message.value = { texte, action, agir, rang: (message.value?.rang ?? 0) + 1 })

async function basculerAgenda(): Promise<void> {
  if (!compte.connectee.value) {
    feuilleCompte.value = true
    return
  }
  if (dansLAgenda.value) {
    await agenda.retirerReunion(id.value)
    annoncer(k('agenda.retiree'), k('agenda.annuler'), () => {
      message.value = null
      void agenda.ajouterReunion(id.value)
    })
  } else {
    await agenda.ajouterReunion(id.value)
    annoncer(k('agenda.ajoutee'))
  }
}

useHead({ title: computed(() => reunion.value?.title ?? k('titre')) })
</script>

<template>
  <GnEcran
    :titre="reunion?.title ?? k('titre')"
    :sous-titre="reunion ? k('fil') : undefined"
    :retour="retour"
    :onglets="false"
    :ce-qui-se-lit="k('hors-connexion')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="5" :libelle="k('chargement')" class="gn-reseau__attente" />

    <GnEtatVide
      v-else-if="sansEdition"
      picto="nego"
      :titre="t('guide-nego.negociations-fiche.sans-edition.titre')"
      :texte="t('guide-nego.negociations-fiche.sans-edition.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatVide
      v-else-if="!lue && !enLigne"
      picto="wifi-off"
      :titre="t('guide-nego.negociations-fiche.jamais-lue.titre')"
      :texte="t('guide-nego.negociations-fiche.jamais-lue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatErreur
      v-else-if="!lue"
      :titre="t('guide-nego.negociations-fiche.erreur.titre')"
      :texte="t('guide-nego.negociations-fiche.erreur.texte')"
      :sortie="t('guide-nego.negociations-fiche.erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!reunion"
      picto="diamond"
      :titre="k('inconnue.titre')"
      :texte="k('inconnue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <div v-else class="gn-reseau">
      <div class="gn-reseau__tete">
        <GnMarqueEtat etat="non-annoncee" :libelle="marque" />
      </div>

      <div class="gn-reseau__corps">
        <h2 class="gn-reseau__intertitre">{{ k('intertitre') }}</h2>
        <dl class="gn-reseau__lignes">
          <div v-if="quand" class="gn-reseau__ligne">
            <dt>{{ k('cle.heure') }}</dt>
            <dd><GnValeurChangee :valeur="quand.valeur" :precision="quand.precision" /></dd>
          </div>
          <div class="gn-reseau__ligne">
            <dt>{{ k('cle.lieu') }}</dt>
            <dd><GnValeurChangee :valeur="reunion.venue ?? k('sans-lieu')" forte /></dd>
          </div>
          <div class="gn-reseau__ligne">
            <dt>{{ k('cle.thematique') }}</dt>
            <dd class="gn-reseau__valeur">{{ thematique }}</dd>
          </div>
        </dl>
        <p class="gn-reseau__phrase">
          <GnPicto nom="shield-check" :taille="20" />
          <span>{{ k('phrase') }}</span>
        </p>
      </div>

      <div class="gn-reseau__actions">
        <GnBouton v-if="dansLAgenda" variante="secondaire" picto="check" actif @clic="basculerAgenda">
          {{ k('agenda.dedans') }}
        </GnBouton>
        <GnBouton
          v-else
          picto="calendar"
          :chargement="compte.connectee.value && !agenda.pret.value"
          @clic="basculerAgenda"
        >
          {{ k('agenda.ajouter') }}
        </GnBouton>
      </div>
    </div>

    <GnFeuilleBasse
      v-model="feuilleCompte"
      :titre="t('guide-nego.negociations-fiche.compte.titre')"
      :sous-titre="k('compte')"
    >
      <div class="gn-reseau__sorties-compte">
        <GnBouton vers="/guide-nego/compte" picto="user">{{ t('guide-nego.negociations-fiche.compte.creer') }}</GnBouton>
        <GnBouton variante="secondaire" vers="/guide-nego/connexion">
          {{ t('guide-nego.negociations-fiche.compte.connexion') }}
        </GnBouton>
      </div>
    </GnFeuilleBasse>

    <GnMessageEphemere
      v-if="message"
      :key="message.rang"
      :texte="message.texte"
      :action="message.action"
      @agir="message.agir?.()"
      @fini="message = null"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-reseau) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-reseau {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-reseau__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reseau__tete {
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-reseau__tete .gn-marque-etat {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-reseau__corps {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reseau__intertitre {
  padding: var(--gn-espace-16) 0 6px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-reseau__ligne {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  padding-block: 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-reseau__ligne:last-child {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-reseau__ligne dt {
  flex: none;
  padding-top: 2px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-reseau__ligne dd {
  min-width: 0;
  display: flex;
  justify-content: flex-end;
}

[data-app="guide-nego"] .gn-reseau__ligne dd.gn-reseau__valeur {
  display: block;
  font-weight: var(--gn-graisse-demi-gras);
  text-align: end;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reseau__phrase {
  margin-top: var(--gn-espace-16);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-reseau__phrase .gn-picto {
  flex: none;
}

[data-app="guide-nego"] .gn-reseau__actions {
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

[data-app="guide-nego"] .gn-reseau__sorties-compte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}
</style>
