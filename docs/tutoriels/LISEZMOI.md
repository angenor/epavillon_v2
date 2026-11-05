# Tutoriels vidéo des organisations

Une vidéo courte par geste de l'espace organisation, filmée sur le vrai site avec les données d'exemple (aucune vraie organisation à l'écran), montée avec HyperFrames, voix off ElevenLabs par OpenRouter.

| Vidéo | Dossier | État |
|-------|---------|------|
| Ajouter l'image de couverture | [01-image-de-couverture/](01-image-de-couverture/) | montée le 05/11/2026, 1 min 44 |

## Fabriquer une vidéo

Chaque dossier contient :
- `texte.md` : découpage et texte de la voix off. **La voix lit ce texte mot pour mot** ; les libellés cités doivent être ceux de l'écran.
- `voix/` : un MP3 par scène, modèle `elevenlabs/eleven-v4`, voix `george` (choisie le 05/11), par `POST https://openrouter.ai/api/v1/audio/speech` avec `OPENROUTER_API_KEY` de `.env`. Environ 0,15 $ la vidéo.
- `tournage/` : scripts Playwright. Ils se connectent avec le compte de démonstration (mot de passe lu dans `.env`), capturent chaque étape en 1440 × 810 à 2× et relèvent l'emplacement des boutons dans `reperes.json`.
- `montage/construire.py` : engendre la composition HyperFrames (une sous-composition par scène, sous-titres, sons). Curseur, zooms et cadres se calculent depuis `reperes.json` ; les temps, depuis les pauses de la voix (`ffmpeg -af silencedetect=n=-38dB:d=0.22`).

La vidéo rendue se range dans le dossier de la vidéo (`01-image-de-couverture/tutoriel-image-de-couverture.mp4`), ignorée par Git (`docs/tutoriels/.gitignore`). Les captures (14 Mo) ne sont pas versionnées : on les refait.

## Tourner

1. Copie de travail à part (`git worktree add --detach <dossier> HEAD`), jamais l'arbre principal : `deploy.sh` l'envoie tel quel.
2. Y mettre en scène les données d'exemple, sans rien enregistrer :
   - l'édition « COP31 Belém 2027 » de `mocks/event.ts` devient CdP31, Antalya, `Europe/Istanbul`, novembre 2026 (dates `2027-11-…-03:00` des mocks ramenées en `2026-11-…+03:00`), fuseau par défaut de `mocks/sessions/_shared.ts` aussi ;
   - un logo fictif pour le ROAC (`tournage/logo.mjs`, rattachement `logo` dans `mocks/covers.ts`) ;
   - `mocks/incidents.ts` : `display_until` passé, pour cacher le bandeau d'incident ;
   - la couverture de `PROPOSAL.adaptationCotiere` : retirée pour l'état « avant », remplacée par l'image fabriquée à l'atelier puis par la photo déposée pour les états « après ».
3. `NUXT_PUBLIC_DEMO_PASSWORD=… npx nuxt dev --port 3051` sans `NUXT_PUBLIC_API_BASE`, puis `node passage-a.mjs` (espace, dossier, modèles), `passage-a2.mjs` (atelier, enregistrement, image produite), `passage-a3.mjs` (dépôt d'une photo), `passage-b.mjs modele|photo` (états après).

## Monter

```bash
npx hyperframes init video --example blank   # une fois
# assets/plans (captures en JPEG), assets/voix, assets/fonts (Barlow Condensed, Inter du site), logos, clic
python3 construire.py && npx hyperframes check --json && npx hyperframes render -q high -o tutoriel.mp4
```

La transcription de HyperFrames (Parakeet) a échoué au téléchargement de son modèle le 05/11 : les sous-titres sont calés phrase par phrase sur les pauses mesurées.
