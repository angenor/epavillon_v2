<script setup lang="ts">
import type { FormFieldType, RegistrationFormField } from '~/types/programme/registration'
import type { I18nText } from '~/types/shared'
import {
  champsAffiches,
  champsSansReponse,
  consentementRequis,
  reponsesDeLaSaisie,
  saisieInitiale,
  type SaisieDuFormulaire,
} from '~/utils/guide-nego/pavillon'

/**
 * Le formulaire d'inscription à une activité du Pavillon, dans une feuille basse (R4) :
 * un rendu par type de champ du modèle, et aucun autre. Libellés, aides et choix sont des
 * données de la base. Le serveur reste juge des formats : son refus s'affiche tel quel
 * près du champ qu'il nomme.
 */
const props = withDefaults(
  defineProps<{
    champs: RegistrationFormField[]
    titre: string
    paysIso2?: string | null
    envoi?: boolean
    /** Les refus du serveur, par code de champ. */
    erreursServeur?: Record<string, string>
    /** Un refus qui ne nomme aucun champ du formulaire. */
    refus?: string | null
    attente?: boolean
  }>(),
  { paysIso2: null, envoi: false, erreursServeur: () => ({}), refus: null, attente: false },
)

const emit = defineEmits<{ envoyer: [reponses: Record<string, unknown>, consentement: boolean] }>()
const ouverte = defineModel<boolean>({ required: true })

const { t } = useI18n()
const { tr } = useI18nText()
const pays = useGnPays()
const k = (cle: string, params: Record<string, unknown> = {}) => t(`gn-formulaire-inscription.${cle}`, params)

const saisie = ref<SaisieDuFormulaire>({})
const consentement = ref(false)
const erreursLocales = ref<Record<string, string>>({})
const CLE_CONSENTEMENT = '__consentement'

watch(
  ouverte,
  (voulue) => {
    if (!voulue) return
    saisie.value = saisieInitiale(props.champs, props.paysIso2)
    consentement.value = false
    erreursLocales.value = {}
    void pays.assurer()
  },
  { immediate: true },
)

const champs = computed(() => champsAffiches(props.champs))
const sensible = computed(() => champs.value.some((c) => c.is_sensitive))

const RENDU: Record<FormFieldType, 'ligne' | 'zone' | 'choix' | 'cases' | 'case' | 'pays'> = {
  text: 'ligne',
  email: 'ligne',
  phone: 'ligne',
  number: 'ligne',
  date: 'ligne',
  long_text: 'zone',
  single_choice: 'choix',
  taxonomy_term: 'choix',
  multiple_choice: 'cases',
  boolean: 'case',
  country: 'pays',
}

const TYPE_DE_SAISIE = { text: 'text', email: 'email', phone: 'tel', number: 'number', date: 'date' } as const
const typeDeSaisie = (c: RegistrationFormField) =>
  TYPE_DE_SAISIE[c.field_type as keyof typeof TYPE_DE_SAISIE] ?? 'text'
const AUTOCOMPLETE: Partial<Record<FormFieldType, string>> = { email: 'email', phone: 'tel' }

const libelle = (c: RegistrationFormField) =>
  c.is_required ? k('obligatoire', { libelle: tr(c.label) }) : tr(c.label)
const aide = (c: RegistrationFormField) => (c.help_text ? tr(c.help_text) : undefined)
const erreur = (code: string) => erreursLocales.value[code] ?? props.erreursServeur[code] ?? undefined

const valeurs = (c: RegistrationFormField): Array<{ value: string; label: I18nText }> =>
  'values' in c.options && Array.isArray(c.options.values) ? c.options.values : []

const maximum = (c: RegistrationFormField) => {
  const max = c.validation.maxLength
  return typeof max === 'number' && max > 0 ? max : 2000
}

const texte = (code: string) => {
  const v = saisie.value[code]
  return typeof v === 'string' ? v : ''
}
const coche = (code: string) => saisie.value[code] === true
const cochees = (code: string) => {
  const v = saisie.value[code]
  return Array.isArray(v) ? v : []
}

function poser(code: string, valeur: string | string[] | boolean): void {
  saisie.value = { ...saisie.value, [code]: valeur }
  if (code in erreursLocales.value) {
    const { [code]: _retiree, ...reste } = erreursLocales.value
    erreursLocales.value = reste
  }
}

function basculerChoix(code: string, valeur: string, voulue: boolean): void {
  const autres = cochees(code).filter((v) => v !== valeur)
  poser(code, voulue ? [...autres, valeur] : autres)
}

function envoyer(): void {
  if (props.envoi) return
  const reponses = reponsesDeLaSaisie(props.champs, saisie.value)
  const erreurs: Record<string, string> = Object.fromEntries(
    champsSansReponse(props.champs, reponses).map((code) => [code, k('requis')]),
  )
  if (consentementRequis(props.champs, reponses) && !consentement.value) erreurs[CLE_CONSENTEMENT] = k('consentement-requis')
  erreursLocales.value = erreurs
  if (Object.keys(erreurs).length) return
  emit('envoyer', reponses, consentement.value)
}
</script>

<template>
  <GnFeuilleBasse v-model="ouverte" :titre="attente ? k('titre-attente') : k('titre')" :sous-titre="titre">
    <form class="gn-formulaire-inscription" novalidate @submit.prevent="envoyer">
      <template v-for="c in champs" :key="c.code">
        <GnZoneTexte
          v-if="RENDU[c.field_type] === 'zone'"
          :model-value="texte(c.code)"
          :libelle="libelle(c)"
          :aide="aide(c)"
          :erreur="erreur(c.code)"
          :maximum="maximum(c)"
          @update:model-value="poser(c.code, $event)"
        />

        <GnChamp
          v-else-if="RENDU[c.field_type] === 'choix' || (RENDU[c.field_type] === 'pays' && pays.liste.value.length)"
          :libelle="libelle(c)"
          :aide="aide(c)"
          :erreur="erreur(c.code)"
        >
          <template #default="{ idSaisie, decritPar, invalide }">
            <select
              :id="idSaisie"
              class="gn-champ__saisie gn-formulaire-inscription__liste"
              :value="texte(c.code)"
              :aria-describedby="decritPar"
              :aria-invalid="invalide"
              :aria-required="c.is_required ? 'true' : undefined"
              @change="poser(c.code, ($event.target as HTMLSelectElement).value)"
            >
              <option value="">{{ k('choisir') }}</option>
              <template v-if="c.field_type === 'country'">
                <option v-for="p in pays.liste.value" :key="p.iso2" :value="p.iso2">{{ p.nom }}</option>
              </template>
              <template v-else>
                <option v-for="o in valeurs(c)" :key="o.value" :value="o.value">{{ tr(o.label) }}</option>
              </template>
            </select>
          </template>
        </GnChamp>

        <div v-else-if="RENDU[c.field_type] === 'case'" class="gn-formulaire-inscription__groupe">
          <GnCase
            :model-value="coche(c.code)"
            :libelle="libelle(c)"
            :detail="aide(c)"
            derniere
            @update:model-value="poser(c.code, $event)"
          />
          <p v-if="erreur(c.code)" class="gn-formulaire-inscription__erreur" role="alert">
            <GnPicto nom="warn" :taille="20" />
            <span>{{ erreur(c.code) }}</span>
          </p>
        </div>

        <fieldset
          v-else-if="RENDU[c.field_type] === 'cases'"
          class="gn-formulaire-inscription__groupe"
          :aria-invalid="erreur(c.code) ? 'true' : undefined"
        >
          <legend class="gn-formulaire-inscription__legende">{{ libelle(c) }}</legend>
          <p v-if="aide(c)" class="gn-formulaire-inscription__aide">{{ aide(c) }}</p>
          <GnCase
            v-for="(o, rang) in valeurs(c)"
            :key="o.value"
            :model-value="cochees(c.code).includes(o.value)"
            :libelle="tr(o.label)"
            :derniere="rang === valeurs(c).length - 1"
            @update:model-value="basculerChoix(c.code, o.value, $event)"
          />
          <p v-if="erreur(c.code)" class="gn-formulaire-inscription__erreur" role="alert">
            <GnPicto nom="warn" :taille="20" />
            <span>{{ erreur(c.code) }}</span>
          </p>
        </fieldset>

        <!-- Pays sans référentiel lu (hors connexion) : le code à deux lettres, prérempli. -->
        <GnChamp
          v-else
          :model-value="texte(c.code)"
          :type="c.field_type === 'country' ? 'text' : typeDeSaisie(c)"
          :libelle="libelle(c)"
          :aide="c.field_type === 'country' ? k('pays-code') : aide(c)"
          :erreur="erreur(c.code)"
          :autocomplete="AUTOCOMPLETE[c.field_type] ?? 'off'"
          :maxlength="c.field_type === 'country' ? 2 : undefined"
          :inputmode="c.field_type === 'number' ? 'decimal' : undefined"
          @update:model-value="poser(c.code, $event)"
        />
      </template>

      <div v-if="sensible" class="gn-formulaire-inscription__consentement">
        <GnCase v-model="consentement" :libelle="k('consentement')" :detail="k('consentement-detail')" derniere />
        <p v-if="erreursLocales[CLE_CONSENTEMENT]" class="gn-formulaire-inscription__erreur" role="alert">
          <GnPicto nom="warn" :taille="20" />
          <span>{{ erreursLocales[CLE_CONSENTEMENT] }}</span>
        </p>
      </div>

      <p v-if="refus" class="gn-formulaire-inscription__erreur" role="alert">
        <GnPicto nom="warn" :taille="20" />
        <span>{{ refus }}</span>
      </p>

      <GnBouton type="submit" :picto="attente ? 'clock' : 'check'" :chargement="envoi">
        {{ attente ? k('envoyer-attente') : k('envoyer') }}
      </GnBouton>
    </form>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-formulaire-inscription {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-formulaire-inscription__liste {
  appearance: auto;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-formulaire-inscription__groupe {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-formulaire-inscription__legende {
  padding: 0;
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-formulaire-inscription__aide {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-formulaire-inscription__groupe[aria-invalid="true"] .gn-formulaire-inscription__legende {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-formulaire-inscription__erreur {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-danger);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-formulaire-inscription__erreur .gn-picto {
  flex: none;
  margin-block-start: 1px;
}

[data-app="guide-nego"] .gn-formulaire-inscription__consentement {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  border-top: var(--gn-filet-1) solid var(--gn-filet-doux);
}
</style>
