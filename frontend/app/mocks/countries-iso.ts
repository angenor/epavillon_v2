/**
 * LA NORME ISO 3166-1, POUR LE SÉLECTEUR DE PAYS DU BACK-OFFICE.
 *
 * ── POURQUOI CE FICHIER EXISTE ──────────────────────────────────────────────
 *
 * `mocks/reference.ts` tient vingt fiches de pays écrites À LA MAIN : celles
 * auxquelles se rattachent les organisations, les personnes et les éditions du jeu
 * d'essai. Elles portent des données que la norme ne donne pas — nom officiel,
 * indicatif téléphonique, code de région — et leur identifiant est déclaré dans
 * `ids.ts`, parce que d'autres fichiers de mocks y renvoient.
 *
 * Les 231 AUTRES n'ont aucune de ces attaches. Elles n'existent que pour qu'un
 * sélecteur soit complet : une conférence des Nations unies se tient là où les
 * Nations unies la placent, et une liste réduite à l'espace francophone
 * obligerait à modifier les données avant chaque édition hors zone. Les écrire à
 * la main coûterait quatre mille lignes de recopie, pour une valeur nulle.
 *
 * ── D'OÙ VIENNENT LES DONNÉES ───────────────────────────────────────────────
 *
 *   · les CODES et le fuseau viennent de la table ci-dessous, qui reprend
 *     `900_seed.sql` § 3 bis — même source, mêmes 249 entités, vérifiées
 *     mécaniquement (trois codes uniques par entité, tout fuseau valide) ;
 *   · les NOMS viennent des données CLDR de l'exécution (`Intl.DisplayNames`),
 *     en français et en anglais. Aucun nom n'est recopié ici : c'est ce qui
 *     évite les deux cent trente et une occasions de faute de frappe, et ce qui
 *     garantit les accents — « Émirats arabes unis », pas « Emirats ».
 *
 * ── CE QUE CE FICHIER NE FAIT PAS ───────────────────────────────────────────
 *
 * IL NE REDÉFINIT AUCUN DES DIX-HUIT PAYS CURÉS. `reference.ts` les place en
 * premier et ce fichier écarte leur code : « République démocratique du Congo »
 * reste la forme protocolaire, là où CLDR écrirait « Congo-Kinshasa ». Le semis
 * SQL fait exactement le même arbitrage, par `ON CONFLICT (iso2) DO NOTHING`.
 *
 * IL N'AFFIRME AUCUN STATUT OIF. `oif_status` vaut `none` sur toute cette liste.
 * L'OIF compte 88 États et gouvernements, la liste évolue à chaque Sommet et fait
 * autorité chez elle : un statut deviné serait une donnée fausse dans un
 * référentiel dont l'IFDD se sert pour ses rapports.
 */

import type { Country } from '~/types/reference'
import { mockUuid } from './ids'

/** `alpha-2|alpha-3|numérique|continent|fuseau` — ISO 3166-1, ordonné par alpha-2. */
const ISO_3166 = [
  'AD|AND|020|europe|Europe/Andorra', 'AE|ARE|784|asia|Asia/Dubai', 'AF|AFG|004|asia|Asia/Kabul',
  'AG|ATG|028|north_america|America/Antigua', 'AI|AIA|660|north_america|America/Anguilla', 'AL|ALB|008|europe|Europe/Tirane',
  'AM|ARM|051|asia|Asia/Yerevan', 'AO|AGO|024|africa|Africa/Luanda', 'AQ|ATA|010|antarctica|Antarctica/Casey',
  'AR|ARG|032|south_america|America/Argentina/Buenos_Aires', 'AS|ASM|016|oceania|Pacific/Pago_Pago', 'AT|AUT|040|europe|Europe/Vienna',
  'AU|AUS|036|oceania|Australia/Sydney', 'AW|ABW|533|north_america|America/Aruba', 'AX|ALA|248|europe|Europe/Mariehamn',
  'AZ|AZE|031|asia|Asia/Baku', 'BA|BIH|070|europe|Europe/Sarajevo', 'BB|BRB|052|north_america|America/Barbados',
  'BD|BGD|050|asia|Asia/Dhaka', 'BE|BEL|056|europe|Europe/Brussels', 'BF|BFA|854|africa|Africa/Ouagadougou',
  'BG|BGR|100|europe|Europe/Sofia', 'BH|BHR|048|asia|Asia/Bahrain', 'BI|BDI|108|africa|Africa/Bujumbura',
  'BJ|BEN|204|africa|Africa/Porto-Novo', 'BL|BLM|652|north_america|America/St_Barthelemy', 'BM|BMU|060|north_america|Atlantic/Bermuda',
  'BN|BRN|096|asia|Asia/Brunei', 'BO|BOL|068|south_america|America/La_Paz', 'BQ|BES|535|north_america|America/Kralendijk',
  'BR|BRA|076|south_america|America/Sao_Paulo', 'BS|BHS|044|north_america|America/Nassau', 'BT|BTN|064|asia|Asia/Thimphu',
  'BV|BVT|074|antarctica|UTC', 'BW|BWA|072|africa|Africa/Gaborone', 'BY|BLR|112|europe|Europe/Minsk',
  'BZ|BLZ|084|north_america|America/Belize', 'CA|CAN|124|north_america|America/Toronto', 'CC|CCK|166|asia|Indian/Cocos',
  'CD|COD|180|africa|Africa/Kinshasa', 'CF|CAF|140|africa|Africa/Bangui', 'CG|COG|178|africa|Africa/Brazzaville',
  'CH|CHE|756|europe|Europe/Zurich', 'CI|CIV|384|africa|Africa/Abidjan', 'CK|COK|184|oceania|Pacific/Rarotonga',
  'CL|CHL|152|south_america|America/Santiago', 'CM|CMR|120|africa|Africa/Douala', 'CN|CHN|156|asia|Asia/Shanghai',
  'CO|COL|170|south_america|America/Bogota', 'CR|CRI|188|north_america|America/Costa_Rica', 'CU|CUB|192|north_america|America/Havana',
  'CV|CPV|132|africa|Atlantic/Cape_Verde', 'CW|CUW|531|north_america|America/Curacao', 'CX|CXR|162|asia|Indian/Christmas',
  'CY|CYP|196|asia|Asia/Nicosia', 'CZ|CZE|203|europe|Europe/Prague', 'DE|DEU|276|europe|Europe/Berlin',
  'DJ|DJI|262|africa|Africa/Djibouti', 'DK|DNK|208|europe|Europe/Copenhagen', 'DM|DMA|212|north_america|America/Dominica',
  'DO|DOM|214|north_america|America/Santo_Domingo', 'DZ|DZA|012|africa|Africa/Algiers', 'EC|ECU|218|south_america|America/Guayaquil',
  'EE|EST|233|europe|Europe/Tallinn', 'EG|EGY|818|africa|Africa/Cairo', 'EH|ESH|732|africa|Africa/El_Aaiun',
  'ER|ERI|232|africa|Africa/Asmara', 'ES|ESP|724|europe|Europe/Madrid', 'ET|ETH|231|africa|Africa/Addis_Ababa',
  'FI|FIN|246|europe|Europe/Helsinki', 'FJ|FJI|242|oceania|Pacific/Fiji', 'FK|FLK|238|south_america|Atlantic/Stanley',
  'FM|FSM|583|oceania|Pacific/Chuuk', 'FO|FRO|234|europe|Atlantic/Faroe', 'FR|FRA|250|europe|Europe/Paris',
  'GA|GAB|266|africa|Africa/Libreville', 'GB|GBR|826|europe|Europe/London', 'GD|GRD|308|north_america|America/Grenada',
  'GE|GEO|268|asia|Asia/Tbilisi', 'GF|GUF|254|south_america|America/Cayenne', 'GG|GGY|831|europe|Europe/Guernsey',
  'GH|GHA|288|africa|Africa/Accra', 'GI|GIB|292|europe|Europe/Gibraltar', 'GL|GRL|304|north_america|America/Nuuk',
  'GM|GMB|270|africa|Africa/Banjul', 'GN|GIN|324|africa|Africa/Conakry', 'GP|GLP|312|north_america|America/Guadeloupe',
  'GQ|GNQ|226|africa|Africa/Malabo', 'GR|GRC|300|europe|Europe/Athens', 'GS|SGS|239|antarctica|Atlantic/South_Georgia',
  'GT|GTM|320|north_america|America/Guatemala', 'GU|GUM|316|oceania|Pacific/Guam', 'GW|GNB|624|africa|Africa/Bissau',
  'GY|GUY|328|south_america|America/Guyana', 'HK|HKG|344|asia|Asia/Hong_Kong', 'HM|HMD|334|antarctica|UTC',
  'HN|HND|340|north_america|America/Tegucigalpa', 'HR|HRV|191|europe|Europe/Zagreb', 'HT|HTI|332|north_america|America/Port-au-Prince',
  'HU|HUN|348|europe|Europe/Budapest', 'ID|IDN|360|asia|Asia/Jakarta', 'IE|IRL|372|europe|Europe/Dublin',
  'IL|ISR|376|asia|Asia/Jerusalem', 'IM|IMN|833|europe|Europe/Isle_of_Man', 'IN|IND|356|asia|Asia/Kolkata',
  'IO|IOT|086|asia|Indian/Chagos', 'IQ|IRQ|368|asia|Asia/Baghdad', 'IR|IRN|364|asia|Asia/Tehran',
  'IS|ISL|352|europe|Atlantic/Reykjavik', 'IT|ITA|380|europe|Europe/Rome', 'JE|JEY|832|europe|Europe/Jersey',
  'JM|JAM|388|north_america|America/Jamaica', 'JO|JOR|400|asia|Asia/Amman', 'JP|JPN|392|asia|Asia/Tokyo',
  'KE|KEN|404|africa|Africa/Nairobi', 'KG|KGZ|417|asia|Asia/Bishkek', 'KH|KHM|116|asia|Asia/Phnom_Penh',
  'KI|KIR|296|oceania|Pacific/Tarawa', 'KM|COM|174|africa|Indian/Comoro', 'KN|KNA|659|north_america|America/St_Kitts',
  'KP|PRK|408|asia|Asia/Pyongyang', 'KR|KOR|410|asia|Asia/Seoul', 'KW|KWT|414|asia|Asia/Kuwait',
  'KY|CYM|136|north_america|America/Cayman', 'KZ|KAZ|398|asia|Asia/Almaty', 'LA|LAO|418|asia|Asia/Vientiane',
  'LB|LBN|422|asia|Asia/Beirut', 'LC|LCA|662|north_america|America/St_Lucia', 'LI|LIE|438|europe|Europe/Vaduz',
  'LK|LKA|144|asia|Asia/Colombo', 'LR|LBR|430|africa|Africa/Monrovia', 'LS|LSO|426|africa|Africa/Maseru',
  'LT|LTU|440|europe|Europe/Vilnius', 'LU|LUX|442|europe|Europe/Luxembourg', 'LV|LVA|428|europe|Europe/Riga',
  'LY|LBY|434|africa|Africa/Tripoli', 'MA|MAR|504|africa|Africa/Casablanca', 'MC|MCO|492|europe|Europe/Monaco',
  'MD|MDA|498|europe|Europe/Chisinau', 'ME|MNE|499|europe|Europe/Podgorica', 'MF|MAF|663|north_america|America/Marigot',
  'MG|MDG|450|africa|Indian/Antananarivo', 'MH|MHL|584|oceania|Pacific/Majuro', 'MK|MKD|807|europe|Europe/Skopje',
  'ML|MLI|466|africa|Africa/Bamako', 'MM|MMR|104|asia|Asia/Yangon', 'MN|MNG|496|asia|Asia/Ulaanbaatar',
  'MO|MAC|446|asia|Asia/Macau', 'MP|MNP|580|oceania|Pacific/Saipan', 'MQ|MTQ|474|north_america|America/Martinique',
  'MR|MRT|478|africa|Africa/Nouakchott', 'MS|MSR|500|north_america|America/Montserrat', 'MT|MLT|470|europe|Europe/Malta',
  'MU|MUS|480|africa|Indian/Mauritius', 'MV|MDV|462|asia|Indian/Maldives', 'MW|MWI|454|africa|Africa/Blantyre',
  'MX|MEX|484|north_america|America/Mexico_City', 'MY|MYS|458|asia|Asia/Kuala_Lumpur', 'MZ|MOZ|508|africa|Africa/Maputo',
  'NA|NAM|516|africa|Africa/Windhoek', 'NC|NCL|540|oceania|Pacific/Noumea', 'NE|NER|562|africa|Africa/Niamey',
  'NF|NFK|574|oceania|Pacific/Norfolk', 'NG|NGA|566|africa|Africa/Lagos', 'NI|NIC|558|north_america|America/Managua',
  'NL|NLD|528|europe|Europe/Amsterdam', 'NO|NOR|578|europe|Europe/Oslo', 'NP|NPL|524|asia|Asia/Kathmandu',
  'NR|NRU|520|oceania|Pacific/Nauru', 'NU|NIU|570|oceania|Pacific/Niue', 'NZ|NZL|554|oceania|Pacific/Auckland',
  'OM|OMN|512|asia|Asia/Muscat', 'PA|PAN|591|north_america|America/Panama', 'PE|PER|604|south_america|America/Lima',
  'PF|PYF|258|oceania|Pacific/Tahiti', 'PG|PNG|598|oceania|Pacific/Port_Moresby', 'PH|PHL|608|asia|Asia/Manila',
  'PK|PAK|586|asia|Asia/Karachi', 'PL|POL|616|europe|Europe/Warsaw', 'PM|SPM|666|north_america|America/Miquelon',
  'PN|PCN|612|oceania|Pacific/Pitcairn', 'PR|PRI|630|north_america|America/Puerto_Rico', 'PS|PSE|275|asia|Asia/Hebron',
  'PT|PRT|620|europe|Europe/Lisbon', 'PW|PLW|585|oceania|Pacific/Palau', 'PY|PRY|600|south_america|America/Asuncion',
  'QA|QAT|634|asia|Asia/Qatar', 'RE|REU|638|africa|Indian/Reunion', 'RO|ROU|642|europe|Europe/Bucharest',
  'RS|SRB|688|europe|Europe/Belgrade', 'RU|RUS|643|europe|Europe/Moscow', 'RW|RWA|646|africa|Africa/Kigali',
  'SA|SAU|682|asia|Asia/Riyadh', 'SB|SLB|090|oceania|Pacific/Guadalcanal', 'SC|SYC|690|africa|Indian/Mahe',
  'SD|SDN|729|africa|Africa/Khartoum', 'SE|SWE|752|europe|Europe/Stockholm', 'SG|SGP|702|asia|Asia/Singapore',
  'SH|SHN|654|africa|Atlantic/St_Helena', 'SI|SVN|705|europe|Europe/Ljubljana', 'SJ|SJM|744|europe|Arctic/Longyearbyen',
  'SK|SVK|703|europe|Europe/Bratislava', 'SL|SLE|694|africa|Africa/Freetown', 'SM|SMR|674|europe|Europe/San_Marino',
  'SN|SEN|686|africa|Africa/Dakar', 'SO|SOM|706|africa|Africa/Mogadishu', 'SR|SUR|740|south_america|America/Paramaribo',
  'SS|SSD|728|africa|Africa/Juba', 'ST|STP|678|africa|Africa/Sao_Tome', 'SV|SLV|222|north_america|America/El_Salvador',
  'SX|SXM|534|north_america|America/Lower_Princes', 'SY|SYR|760|asia|Asia/Damascus', 'SZ|SWZ|748|africa|Africa/Mbabane',
  'TC|TCA|796|north_america|America/Grand_Turk', 'TD|TCD|148|africa|Africa/Ndjamena', 'TF|ATF|260|antarctica|Indian/Kerguelen',
  'TG|TGO|768|africa|Africa/Lome', 'TH|THA|764|asia|Asia/Bangkok', 'TJ|TJK|762|asia|Asia/Dushanbe',
  'TK|TKL|772|oceania|Pacific/Fakaofo', 'TL|TLS|626|asia|Asia/Dili', 'TM|TKM|795|asia|Asia/Ashgabat',
  'TN|TUN|788|africa|Africa/Tunis', 'TO|TON|776|oceania|Pacific/Tongatapu', 'TR|TUR|792|asia|Europe/Istanbul',
  'TT|TTO|780|north_america|America/Port_of_Spain', 'TV|TUV|798|oceania|Pacific/Funafuti', 'TW|TWN|158|asia|Asia/Taipei',
  'TZ|TZA|834|africa|Africa/Dar_es_Salaam', 'UA|UKR|804|europe|Europe/Kyiv', 'UG|UGA|800|africa|Africa/Kampala',
  'UM|UMI|581|oceania|Pacific/Wake', 'US|USA|840|north_america|America/New_York', 'UY|URY|858|south_america|America/Montevideo',
  'UZ|UZB|860|asia|Asia/Tashkent', 'VA|VAT|336|europe|Europe/Vatican', 'VC|VCT|670|north_america|America/St_Vincent',
  'VE|VEN|862|south_america|America/Caracas', 'VG|VGB|092|north_america|America/Tortola', 'VI|VIR|850|north_america|America/St_Thomas',
  'VN|VNM|704|asia|Asia/Ho_Chi_Minh', 'VU|VUT|548|oceania|Pacific/Efate', 'WF|WLF|876|oceania|Pacific/Wallis',
  'WS|WSM|882|oceania|Pacific/Apia', 'YE|YEM|887|asia|Asia/Aden', 'YT|MYT|175|africa|Indian/Mayotte',
  'ZA|ZAF|710|africa|Africa/Johannesburg', 'ZM|ZMB|894|africa|Africa/Lusaka', 'ZW|ZWE|716|africa|Africa/Harare',
]

/** Drapeau composé depuis l'alpha-2 : deux indicateurs régionaux Unicode. */
function flagOf(alpha2: string): string {
  return String.fromCodePoint(...[...alpha2].map((c) => 0x1f1e6 + c.charCodeAt(0) - 65))
}

/**
 * Nom du pays dans une locale, par les données CLDR de l'exécution.
 *
 * Le repli sur l'alpha-2 n'est pas décoratif : une exécution dépourvue de données
 * ICU complètes rend le code tel quel plutôt que de lever, et le sélecteur reste
 * utilisable — c'est la leçon du fuseau `Europe/Geneva`, dont l'exception avait
 * emporté toute une liste.
 */
function displayName(alpha2: string, locale: 'fr' | 'en'): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'region' }).of(alpha2) ?? alpha2
  } catch {
    return alpha2
  }
}

/**
 * Les pays de la norme qui ne sont pas déjà curés à la main.
 *
 * Numérotés à partir de 100 dans la famille `7010` : les vingt fiches curées
 * occupent 1 à 20, et rien ne doit les recouvrir.
 */
export function isoCountriesExcluding(curatedAlpha2: string[]): Country[] {
  const curated = new Set(curatedAlpha2)

  return ISO_3166.map((row) => row.split('|'))
    .filter(([alpha2]) => alpha2 !== undefined && !curated.has(alpha2))
    .map(([alpha2, alpha3, numeric, continent, timezone], index) => ({
      id: mockUuid('7010', 100 + index),
      iso2: alpha2!,
      iso3: alpha3!,
      numeric_code: numeric!,
      name: { fr: displayName(alpha2!, 'fr'), en: displayName(alpha2!, 'en') },
      official_name: null,
      // Colonne GÉNÉRÉE en base (`platform.normalize_label(name->>'fr')`) : on la
      // dérive de la même façon plutôt que de la laisser vide, sans quoi une
      // recherche par nom ne trouverait que les vingt fiches curées.
      name_normalized: displayName(alpha2!, 'fr')
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, ' ')
        .trim(),
      region_code: null,
      continent: continent as Country['continent'],
      oif_status: 'none' as const,
      default_timezone: timezone!,
      calling_code: null,
      flag_emoji: flagOf(alpha2!),
      is_active: true,
      created_at: '2026-01-12T09:00:00Z',
      updated_at: '2026-01-12T09:00:00Z',
    }))
}
