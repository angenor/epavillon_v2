/* Commandes de maquette : état de la journée simulé, thème, ajout à l'agenda.
   La page déclare sur <html> : data-date="2026-11-13", data-heure-j="11:40", data-tz="Europe/Istanbul". */
(function () {
  const root = document.documentElement
  const params = new URLSearchParams(location.search)

  const sprite = `<svg xmlns="http://www.w3.org/2000/svg" style="display:none">
    <symbol id="i-cal" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18"/></symbol>
    <symbol id="i-cal-plus" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18M12 14v5M9.5 16.5h5"/></symbol>
    <symbol id="i-pin" viewBox="0 0 24 24"><path d="M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0Z"/><circle cx="12" cy="10" r="3"/></symbol>
    <symbol id="i-clock" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></symbol>
    <symbol id="i-right" viewBox="0 0 24 24"><path d="M5 12h14M12 5l7 7-7 7"/></symbol>
    <symbol id="i-left" viewBox="0 0 24 24"><path d="M19 12H5M12 19l-7-7 7-7"/></symbol>
    <symbol id="i-ext" viewBox="0 0 24 24"><path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></symbol>
    <symbol id="i-book" viewBox="0 0 24 24"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20V2H6.5A2.5 2.5 0 0 0 4 4.5v15Z"/><path d="M4 19.5A2.5 2.5 0 0 0 6.5 22H20v-5"/></symbol>
    <symbol id="i-doc" viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z"/><path d="M14 2v6h6M16 13H8M16 17H8M10 9H8"/></symbol>
    <symbol id="i-users" viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/></symbol>
    <symbol id="i-menu" viewBox="0 0 24 24"><path d="M3 6h18M3 12h18M3 18h18"/></symbol>
    <symbol id="i-share" viewBox="0 0 24 24"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="m8.6 13.5 6.8 4M15.4 6.5l-6.8 4"/></symbol>
    <symbol id="i-play" viewBox="0 0 24 24"><path d="m6 4 14 8-14 8Z"/></symbol>
    <symbol id="i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></symbol>
  </svg>`
  document.body.insertAdjacentHTML('afterbegin', sprite)

  const minutes = (hhmm) => { const [h, m] = hhmm.split(':').map(Number); return h * 60 + m }
  const labels = { avant: 'À venir', jour: 'En cours', apres: 'Terminée' }
  const pageClass = { avant: 'etat-upcoming', jour: 'etat-ongoing', apres: 'etat-past' }

  function applyState(etat) {
    root.dataset.etat = etat
    const now = etat === 'avant' ? -1 : etat === 'apres' ? 24 * 60 : minutes(root.dataset.heureJ)
    document.querySelectorAll('[data-etat-page]').forEach((el) => {
      el.className = `etat ${pageClass[etat]}`
      el.textContent = el.dataset[`label${etat[0].toUpperCase()}${etat.slice(1)}`] || labels[etat]
    })
    document.querySelectorAll('[data-debut]').forEach((row) => {
      const start = minutes(row.dataset.debut)
      const end = minutes(row.dataset.fin || row.dataset.debut)
      const state = now >= end ? 'past' : now >= start ? 'current' : 'upcoming'
      row.classList.toggle('is-past', state === 'past')
      row.classList.toggle('is-current', state === 'current')
      row.classList.toggle('is-upcoming', state === 'upcoming')
      const badge = row.querySelector('.etat-ligne')
      if (!badge) return
      badge.hidden = state === 'upcoming'
      if (state === 'past') { badge.className = 'etat etat-past etat-ligne'; badge.textContent = 'Terminé' }
      if (state === 'current') {
        const live = row.hasAttribute('data-direct')
        badge.className = `etat ${live ? 'etat-live' : 'etat-ongoing'} etat-ligne`
        badge.textContent = live ? 'En direct' : 'En cours'
      }
    })
    document.querySelectorAll('[data-maintenant]').forEach((el) => { el.textContent = root.dataset.heureJ })
    document.querySelectorAll('[data-maq-etat]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.maqEtat === etat)))
  }

  function applyTheme(theme) {
    root.dataset.theme = theme
    document.querySelectorAll('[data-maq-theme]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.maqTheme === theme)))
  }

  // Fichier .ics réel : le bouton « Ajouter à mon agenda » fonctionne dans la maquette.
  document.addEventListener('click', (event) => {
    const button = event.target.closest('[data-ics]')
    if (!button) return
    const d = root.dataset.date.replaceAll('-', '')
    const t = (hhmm) => hhmm.replace(':', '') + '00'
    const esc = (s) => s.replace(/([,;])/g, '\\$1')
    const ics = [
      'BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//IFDD//ePavillon maquette//FR', 'BEGIN:VEVENT',
      `UID:${d}-${button.dataset.ics}@epavillon`, `DTSTAMP:20261030T120000Z`,
      `DTSTART;TZID=${root.dataset.tz}:${d}T${t(button.dataset.debut)}`,
      `DTEND;TZID=${root.dataset.tz}:${d}T${t(button.dataset.fin)}`,
      `SUMMARY:${esc(button.dataset.titre)}`, `LOCATION:${esc(button.dataset.lieu)}`,
      'END:VEVENT', 'END:VCALENDAR',
    ].join('\r\n')
    const link = document.createElement('a')
    link.href = URL.createObjectURL(new Blob([ics], { type: 'text/calendar' }))
    link.download = `${button.dataset.ics}.ics`
    link.click()
  })

  if (!params.has('nu')) {
    const bar = document.createElement('div')
    bar.className = 'maq-barre'
    bar.innerHTML = `<b>Maquette</b>
      <span>Moment :</span>
      <button type="button" data-maq-etat="avant">Avant</button>
      <button type="button" data-maq-etat="jour">Jour J, ${root.dataset.heureJ}</button>
      <button type="button" data-maq-etat="apres">Après</button>
      <span>Thème :</span>
      <button type="button" data-maq-theme="light">Clair</button>
      <button type="button" data-maq-theme="dark">Sombre</button>
      <a href="${root.dataset.sommaire || '../index.html'}">Sommaire</a>`
    document.body.append(bar)
    bar.addEventListener('click', (event) => {
      const b = event.target.closest('button')
      if (!b) return
      if (b.dataset.maqEtat) applyState(b.dataset.maqEtat)
      if (b.dataset.maqTheme) applyTheme(b.dataset.maqTheme)
    })
    const style = document.createElement('style')
    style.textContent = `.maq-barre{position:fixed;left:8px;bottom:8px;z-index:50;display:flex;flex-wrap:wrap;align-items:center;gap:6px;max-width:calc(100vw - 16px);padding:6px 10px;border:2px dashed #e63132;border-radius:6px;background:#fff;color:#231f20;font:13px/1.2 ui-monospace,monospace;box-shadow:0 4px 10px rgb(0 0 0/.15)}
      .maq-barre button,.maq-barre a{min-height:28px;padding:0 8px;border:1px solid #888785;border-radius:4px;background:#fff;color:#231f20;font:inherit;cursor:pointer;text-decoration:none;display:inline-flex;align-items:center}
      .maq-barre [aria-pressed="true"]{background:#231f20;color:#fff}`
    document.head.append(style)
  }

  applyTheme(params.get('theme') || 'light')
  applyState(params.get('etat') || 'avant')
})()
