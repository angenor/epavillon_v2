<script setup lang="ts">
/**
 * Se lit dans un navigateur, avant l'application. En mode installé elle n'a plus
 * d'objet et renvoie à l'accueil.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { installee, installer } = useGnInstallation()

/** L'adresse à retaper sur un autre téléphone : celle qu'on a sous les yeux. */
const adresse = ref('')
const etapes = useTemplateRef<HTMLElement>('etapes')

onMounted(() => {
  adresse.value = `${location.host}${useRuntimeConfig().app.baseURL.replace(/\/$/, '')}`
})

watchEffect(() => {
  if (installee.value) void navigateTo('/guide-nego', { replace: true })
})

// Sans proposition du navigateur, le bouton ne ment pas : il mène aux étapes écrites.
async function demander() {
  if (await installer()) return
  etapes.value?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  etapes.value?.focus()
}

useHead({ title: t('guide-nego.installer.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.installer.titre')" :sous-titre="t('guide-nego.installer.sous-titre')" :onglets="false">
    <template #connexion>
      <span class="gn-installer__adresse">{{ adresse }}</span>
    </template>

    <p class="gn-installer__intro">{{ t('guide-nego.installer.intro') }}</p>

    <section ref="etapes" tabindex="-1" :aria-label="t('guide-nego.installer.etapes-libelle')">
      <GnEnteteGroupe :titre="t('guide-nego.installer.android.titre')" />
      <ol class="gn-installer__etapes">
        <li v-for="numero in 3" :key="numero" class="gn-installer__etape">
          <span class="gn-installer__numero" aria-hidden="true">{{ numero }}</span>
          <span>
            <span class="gn-installer__geste">{{ t(`guide-nego.installer.android.etape-${numero}`) }}</span>
            <span v-if="numero !== 2" class="gn-installer__precision">
              {{ t(`guide-nego.installer.android.etape-${numero}-precision`) }}
            </span>
          </span>
        </li>
      </ol>

      <GnEnteteGroupe :titre="t('guide-nego.installer.iphone.titre')" />
      <p class="gn-installer__etape">
        <GnPicto nom="share" class="gn-installer__partager" />
        <span class="gn-installer__geste">{{ t('guide-nego.installer.iphone.etape') }}</span>
      </p>
    </section>

    <p class="gn-installer__avant">{{ t('guide-nego.installer.avant-la-salle') }}</p>

    <div class="gn-installer__sorties">
      <GnBouton variante="principal" picto="download" @clic="demander">
        {{ t('guide-nego.installer.installer') }}
      </GnBouton>
      <GnBouton variante="discret" vers="/guide-nego">{{ t('guide-nego.installer.continuer') }}</GnBouton>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-installer__adresse {
  display: block;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-installer__intro {
  padding-top: var(--gn-espace-16);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-installer__etapes {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-installer__etape {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-installer__numero {
  flex: none;
  width: var(--gn-carre-etape);
  height: var(--gn-carre-etape);
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--gn-titre);
  color: var(--gn-accent-inv);
  font-size: var(--gn-taille-20);
  font-weight: var(--gn-graisse-gras);
  line-height: 1;
}

[data-app="guide-nego"][data-theme="sombre"] .gn-installer__numero {
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-installer__partager {
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-installer__geste {
  display: block;
  font-size: var(--gn-taille-17);
  line-height: 1.3;
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-installer__precision {
  display: block;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-installer__avant {
  margin-top: var(--gn-entre-blocs);
  padding: var(--gn-espace-12) var(--gn-espace-16);
  border: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-installer__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
}
</style>
