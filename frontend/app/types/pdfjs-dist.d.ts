// pdfjs-dist ne type que `pdf.mjs` ; la version minifiée expose la même surface.
declare module 'pdfjs-dist/legacy/build/pdf.min.mjs' {
  export * from 'pdfjs-dist/legacy/build/pdf.mjs'
}
