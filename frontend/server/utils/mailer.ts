/**
 * Envoi SMTP — le seul endroit du dépôt qui parle à un serveur de courriel.
 *
 * Contrainte d'hébergement du 20/08 : l'API et le site vivent sur deux
 * serveurs, et seul celui du site a le droit d'émettre. L'API compose le
 * message et le remet ici ; ce fichier ne fait que transporter.
 *
 * Il est GÉNÉRIQUE, et il doit le rester : il reçoit un destinataire, un sujet
 * et un corps, et il envoie. Il ne connaît aucun cas particulier — ni
 * inscription, ni réinitialisation, ni convocation — et n'en connaîtra jamais.
 * La composition appartient au module qui déclenche l'envoi, et passera en B6
 * aux modèles administrables de `engagement.message_templates`.
 *
 * Il est aussi TEMPORAIRE : le jour où le serveur de l'API obtient le droit
 * d'émettre, l'envoi est réécrit en Rust et ce fichier disparaît.
 */
import { createTransport, type Transporter } from 'nodemailer'

export interface MailMessage {
  message_id: string
  to: string
  locale: string
  subject: string
  text: string
  html?: string
}

let transporteur: Transporter | null = null

function transport(): Transporter {
  if (!transporteur) {
    transporteur = createTransport({
      host: process.env.SMTP_HOST ?? 'localhost',
      port: Number(process.env.SMTP_PORT ?? 1025),
      // Mailpit n'écoute ni en TLS ni avec authentification : exiger l'un ou
      // l'autre rendrait l'environnement local inutilisable.
      secure: false,
      ignoreTLS: true,
    })
  }
  return transporteur
}

export async function envoyer(message: MailMessage): Promise<void> {
  await transport().sendMail({
    from: process.env.SMTP_FROM ?? 'ne-pas-repondre@epavillon.local',
    to: message.to,
    subject: message.subject,
    text: message.text,
    html: message.html,
  })
}

/**
 * Mémoire courte des identifiants pris en charge.
 *
 * Elle absorbe une reprise d'essai après délai d'attente dépassé : le courriel
 * est parti, la réponse s'est perdue, l'API réessaie. Sans elle, la personne
 * recevrait deux fois le même message.
 *
 * **L'identifiant est retenu AVANT l'envoi, pas après.** Le noter après ne
 * protège que d'un doublon séquentiel — or le doublon réel est concurrent : le
 * client de l'API abandonne au bout de quinze secondes et réessaie deux
 * secondes plus tard, pendant que le premier envoi est encore en cours. Mesuré
 * le 20/08, trois exemplaires du même courriel sont partis pour cette seule
 * raison. Node exécute ce test-et-pose sans interruption : la réservation est
 * donc atomique.
 *
 * En mémoire du processus, volontairement : le doublon qu'on veut éviter arrive
 * dans les secondes qui suivent, et une reprise après redémarrage du site est
 * assez rare pour ne pas justifier un stockage partagé.
 */
const DUREE_MEMOIRE_MS = 10 * 60 * 1000
const pris = new Map<string, number>()

/** Faux : un autre appel s'en occupe déjà, ou s'en est déjà occupé. */
export function reserver(messageId: string): boolean {
  const limite = Date.now() - DUREE_MEMOIRE_MS
  for (const [id, instant] of pris) {
    if (instant < limite) pris.delete(id)
  }
  if (pris.has(messageId)) return false
  pris.set(messageId, Date.now())
  return true
}

/**
 * Rend l'identifiant après un envoi qui a échoué : sans cela, la reprise
 * d'essai de l'API serait prise pour un doublon et le courriel ne partirait
 * jamais. Perdre un message vaut pire que le tenter deux fois.
 */
export function liberer(messageId: string): void {
  pris.delete(messageId)
}
