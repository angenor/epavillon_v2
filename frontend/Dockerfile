# Le site, en rendu serveur.
#
# **Trois valeurs sont figées À LA CONSTRUCTION, pas au démarrage** : la base
# sous laquelle le site est servi, l'adresse de l'API et celle du site. La
# première commande le préfixe de chaque URL d'asset émise dans le HTML ; la
# changer après coup ne les réécrirait pas. Reconstruire est donc le seul moyen
# de déplacer le site d'un préfixe à un autre — c'est voulu, et c'est ce qui
# rend le jour de la bascule vers la racine explicite.

FROM node:24-slim AS build

WORKDIR /src

# Les dépendances d'abord : elles ne changent qu'au fichier de verrouillage, et
# la couche se réutilise à toutes les constructions suivantes.
COPY package.json package-lock.json ./
RUN npm ci

COPY . .

ARG NUXT_APP_BASE_URL=/v2/
ARG NUXT_PUBLIC_API_BASE=/v2/api
ARG NUXT_PUBLIC_SITE_URL
ENV NUXT_APP_BASE_URL=$NUXT_APP_BASE_URL \
    NUXT_PUBLIC_API_BASE=$NUXT_PUBLIC_API_BASE \
    NUXT_PUBLIC_SITE_URL=$NUXT_PUBLIC_SITE_URL

# `npm run build` passe `--dotenv ../.env`, absent ici et sans objet : les
# valeurs viennent des arguments ci-dessus.
RUN npx nuxt build

FROM node:24-slim AS runtime

WORKDIR /app
RUN useradd --system --create-home --uid 10001 epavillon
USER epavillon

# Nitro produit un dossier autonome : ni `node_modules`, ni sources.
COPY --from=build /src/.output ./

ENV PORT=3000 \
    HOST=0.0.0.0 \
    NODE_ENV=production

EXPOSE 3000
CMD ["node", "server/index.mjs"]
