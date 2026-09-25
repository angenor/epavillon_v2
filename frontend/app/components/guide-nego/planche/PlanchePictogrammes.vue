<script setup lang="ts">
import { NOMS_DE_PICTO as SYMBOLES } from '~/utils/guide-nego/pictogrammes'

/**
 * Section 4 — les symboles du sprite, avec leur nom.
 *
 * Les noms sont des identifiants techniques, pas des textes d'interface : ils ne
 * passent pas par les traductions, et s'écrivent sans le préfixe `gn-` du sprite.
 */
const TAILLES = [16, 18, 20, 24, 26, 40] as const

const TEINTES = ['--gn-picto', '--gn-picto-secondaire', '--gn-picto-document', '--gn-attention', '--gn-danger'] as const

const { t } = useI18n()
const teinte = (jeton: string) => ({ color: `var(${jeton})` })
</script>

<template>
  <GnPlancheSection
    numero="4"
    :titre="t('gn-planche-pictogrammes.titre')"
    :propos="t('gn-planche-pictogrammes.propos')"
  >
    <div class="gn-planche-pictogrammes">
      <p class="gn-planche-note">{{ t('gn-planche-pictogrammes.regle') }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-pictogrammes.famille', { nombre: SYMBOLES.length })"
        :propos="t('gn-planche-pictogrammes.famille-propos')"
      >
        <ul class="gn-planche-grille gn-planche-grille--serree">
          <li v-for="nom in SYMBOLES" :key="nom" class="gn-planche-pictogrammes__case">
            <GnPicto :nom="nom" :taille="24" />
            <span class="gn-planche-jeton">{{ nom }}</span>
          </li>
        </ul>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-pictogrammes.tailles')"
        :propos="t('gn-planche-pictogrammes.tailles-propos')"
      >
        <ul class="gn-planche-pictogrammes__rangee">
          <li v-for="taille in TAILLES" :key="taille" class="gn-planche-pictogrammes__case">
            <GnPicto nom="calendar" :taille="taille" />
            <span class="gn-planche-valeur">{{ taille }}</span>
          </li>
        </ul>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-pictogrammes.couleur')"
        :propos="t('gn-planche-pictogrammes.couleur-propos')"
      >
        <ul class="gn-planche-pictogrammes__rangee">
          <li
            v-for="jeton in TEINTES"
            :key="jeton"
            class="gn-planche-pictogrammes__case"
            :style="teinte(jeton)"
          >
            <GnPicto nom="doc" :taille="24" />
            <span class="gn-planche-jeton">{{ jeton }}</span>
          </li>
        </ul>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-pictogrammes {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-pictogrammes__case {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  padding: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
  text-align: center;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-pictogrammes__rangee {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche-pictogrammes__rangee > .gn-planche-pictogrammes__case {
  flex: 1 1 96px;
}
</style>
