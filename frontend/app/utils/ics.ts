export interface IcsEvent {
  uid: string
  title: string
  startsAt: string
  endsAt: string
  location?: string
  url?: string
  description?: string
}

const stamp = (value: string | number) => new Date(value).toISOString().replace(/[-:]/g, '').replace(/\.\d{3}/, '')

const escape = (value: string) => value.replace(/\\/g, '\\\\').replace(/\n/g, '\\n').replace(/([,;])/g, '\\$1')

/** Lignes de plus de 75 octets repliées, comme l'exige la RFC 5545. */
function fold(line: string): string {
  const bytes = new TextEncoder().encode(line)
  if (bytes.length <= 75) return line
  const parts: string[] = []
  let current = ''
  for (const char of line) {
    if (new TextEncoder().encode(current + char).length > (parts.length ? 74 : 75)) {
      parts.push(current)
      current = char
    } else {
      current += char
    }
  }
  parts.push(current)
  return parts.join('\r\n ')
}

/** Un fichier d'agenda d'un seul événement, en UTC : l'agenda de chacun le remet à son heure. */
export function icsCalendar(event: IcsEvent, now: number = Date.now()): string {
  const lines = [
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//IFDD//ePavillon//FR',
    'CALSCALE:GREGORIAN',
    'BEGIN:VEVENT',
    `UID:${event.uid}`,
    `DTSTAMP:${stamp(now)}`,
    `DTSTART:${stamp(event.startsAt)}`,
    `DTEND:${stamp(event.endsAt)}`,
    `SUMMARY:${escape(event.title)}`,
    event.location ? `LOCATION:${escape(event.location)}` : '',
    event.description ? `DESCRIPTION:${escape(event.description)}` : '',
    event.url ? `URL:${event.url}` : '',
    'END:VEVENT',
    'END:VCALENDAR',
  ]
  return lines.filter(Boolean).map(fold).join('\r\n') + '\r\n'
}
