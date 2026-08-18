/**
 * Typage du composant global `<apexchart>`, posé par le greffon `vue3-apexcharts`.
 *
 * La bibliothèque enregistre son composant sous ce nom au moment de
 * `app.use(VueApexCharts)` : sans cette déclaration, `vue-tsc` ne le connaît pas
 * et chaque graphique remonte une erreur de composant introuvable.
 *
 * On ne redécrit RIEN de sa surface : le typage authentique est celui du paquet,
 * on ne fait que le rattacher au nom global.
 */
import type VueApexCharts from 'vue3-apexcharts'

declare module 'vue' {
  interface GlobalComponents {
    apexchart: typeof VueApexCharts
  }
}

export {}
