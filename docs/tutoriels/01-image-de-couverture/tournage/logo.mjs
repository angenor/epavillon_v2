import { chromium } from 'playwright'
const b = await chromium.launch(); const p = await b.newPage({ viewport: { width: 800, height: 800 } })
await p.setContent(`<body style="margin:0;width:800px;height:800px;display:grid;place-items:center;background:#fff;font-family:'Helvetica Neue',Arial">
<div style="text-align:center">
<svg width="420" height="230" viewBox="0 0 420 230"><circle cx="210" cy="120" r="100" fill="#F2A900"/>
<path d="M0 150 Q52 110 105 150 T210 150 T315 150 T420 150 V230 H0Z" fill="#0B6E99"/>
<path d="M0 185 Q52 150 105 185 T210 185 T315 185 T420 185 V230 H0Z" fill="#0A4F70"/></svg>
<div style="font-size:150px;font-weight:800;letter-spacing:6px;color:#0A4F70;line-height:1;margin-top:10px">ROAC</div></div></body>`)
await p.screenshot({ path: '../tournage/frontend/public/mocks/covers/logo-roac.jpg', type: 'jpeg', quality: 92 }); await b.close()
