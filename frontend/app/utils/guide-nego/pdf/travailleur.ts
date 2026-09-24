// Un module s'évalue après ceux qu'il importe, dans l'ordre : les remplacements d'abord. Pas
// d'import dynamique : pdf.js n'attend pas le « ready » d'un `workerPort`, un message arrivé trop tôt serait perdu.
import './travailleur-remplacements.ts'
import 'pdfjs-dist/legacy/build/pdf.worker.min.mjs'
