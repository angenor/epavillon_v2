/**
 * Route privée de remise du courriel — appelée par l'API, jamais par un
 * navigateur.
 *
 * Elle n'est ni documentée dans l'OpenAPI, ni référencée par une page. Toute
 * requête sans secret valide reçoit **404, jamais 401** : une route privée ne
 * confirme pas son existence.
 *
 * Elle sert TOUS les courriels de la plateforme, et elle est faite pour
 * disparaître — voir `server/utils/mailer.ts`.
 */
import { timingSafeEqual } from 'node:crypto'
import { envoyer, liberer, reserver, type MailMessage } from '../../utils/mailer'

const ENTETE_SECRET = 'x-mail-relay-token'

/** Comparaison à temps constant : la longueur seule ne doit rien apprendre. */
function secretValide(recu: string | undefined, attendu: string): boolean {
  if (!recu || !attendu) return false
  const a = Buffer.from(recu)
  const b = Buffer.from(attendu)
  if (a.length !== b.length) return false
  return timingSafeEqual(a, b)
}

function introuvable() {
  return createError({ statusCode: 404, statusMessage: 'Not Found' })
}

export default defineEventHandler(async (event) => {
  const attendu = process.env.MAIL_RELAY_TOKEN ?? ''
  const recu = getHeader(event, ENTETE_SECRET)

  // Un secret non configuré ferme la route au lieu de l'ouvrir : sans cela,
  // une variable oubliée au déploiement en ferait un relais ouvert.
  if (!attendu || !secretValide(recu, attendu)) throw introuvable()

  const message = await readBody<MailMessage>(event)
  if (!message?.to || !message.subject || !message.text || !message.message_id) {
    throw createError({ statusCode: 422, statusMessage: 'Message incomplet' })
  }

  // Réponse identique à un envoi réel : pour l'API, le message est remis.
  if (!reserver(message.message_id)) {
    return { status: 'duplicate_ignored' }
  }

  try {
    await envoyer(message)
  } catch (erreur) {
    liberer(message.message_id)
    throw erreur
  }

  return { status: 'sent' }
})
