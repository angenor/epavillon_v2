# API et worker — un seul binaire de construction, deux exécutables.
#
# La construction est HORS LIGNE : `.sqlx/` porte le plan de chaque requête,
# préparé par `cargo sqlx prepare`. Sans lui, SQLx voudrait joindre une base
# pendant la compilation — ce qu'un conteneur de construction n'a pas, et ce
# qu'il ne doit pas avoir.
#
# **Après toute modification d'une requête SQL, refaire `make sqlx-prepare`**,
# sinon l'image se construit sur l'ancien plan et échoue à l'exécution.

FROM rust:1.97-slim-bookworm AS build

WORKDIR /src

# `pkg-config` et `libssl-dev` ne sont PAS installés : la pile TLS du projet est
# rustls de bout en bout — `reqwest`, `sqlx` et `lettre` le déclarent chacun.
# Une image qui les installerait masquerait le jour où l'un d'eux repasse à
# OpenSSL sans qu'on l'ait décidé.
COPY . .

ENV SQLX_OFFLINE=true
RUN cargo build --release --locked -p api -p worker

# PDFium, que le worker charge pour extraire les documents de Guide Négo.
# Épinglé sur l'API que vise `pdfium-render` 0.9 (chromium/7881), comme
# `make pdfium` : un binaire plus récent peut renommer un symbole.
FROM debian:bookworm-slim AS pdfium
ARG PDFIUM_VERSION=7881
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && case "$(uname -m)" in x86_64) plateforme=linux-x64 ;; aarch64) plateforme=linux-arm64 ;; *) exit 1 ;; esac \
 && mkdir -p /opt/pdfium \
 && curl -fsSL "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${PDFIUM_VERSION}/pdfium-${plateforme}.tgz" \
    | tar -xz -C /opt/pdfium

FROM debian:bookworm-slim AS runtime

# Les racines de certification du système. `lettre` embarque les siennes par
# `webpki-roots`, mais SQLx et `reqwest` peuvent consulter celles-ci : trois
# cents kilooctets contre une panne TLS au premier appel sortant.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Jamais root : l'API n'écrit aucun fichier, le worker non plus.
RUN useradd --system --create-home --uid 10001 epavillon
USER epavillon

COPY --from=build /src/target/release/api /usr/local/bin/api
COPY --from=build /src/target/release/worker /usr/local/bin/worker
COPY --from=pdfium /opt/pdfium/lib /opt/pdfium/lib
ENV PDFIUM_LIB_PATH=/opt/pdfium/lib

EXPOSE 8080
CMD ["api"]
