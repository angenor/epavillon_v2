/*!
 * bird-cursor - oiseau interactif qui suit le curseur
 *
 * Integration :
 *   <script src="bird-cursor.js" defer
 *           data-scale="0.42" data-near="230" data-idle="550"><\/script>
 *
 * Les composants sur lesquels l'oiseau peut se poser portent data-bird-perch.
 * Un composant qui porte data-bird-say="texte" appelle l'oiseau au survol :
 * il vient s'y poser et le texte apparait dans sa bulle de conversation.
 * API : window.BirdCursor.mount({ scale, near, idle, stiff, damp, gaze, zIndex })
 *       window.BirdCursor.destroy()
 *
 * Etats : le curseur bouge et s'eloigne  -> l'oiseau vole pour le rattraper
 *         le curseur s'arrete            -> il se pose sur le composant le plus proche
 *         le curseur reste pres de lui   -> il reste assis et le suit du regard
 */
(function () {
  'use strict';

  var MARKUP = "<svg id=\"bird-follow\" xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" height=\"100%\" viewBox=\"0 0 1200 800\" preserveAspectRatio=\"none\" style=\"position:fixed;inset:0;width:100%;height:100%;pointer-events:none;overflow:visible\" xmlns:c2pa=\"http://c2pa.org/manifest\"><metadata><c2pa:manifest>AAAWgmp1bWIAAAAeanVtZGMycGEAEQAQgAAAqgA4m3EDYzJwYQAAABZcanVtYgAAAEdqdW1kYzJtYQARABCAAACqADibcQN1cm46YzJwYTpiNTdjZGVkNC02NjNhLTRhOTAtYWNlMi1kZWJjZDI4ZDhhZmYAAAADl2p1bWIAAAApanVtZGMyYXMAEQAQgAAAqgA4m3EDYzJwYS5hc3NlcnRpb25zAAAAALxqdW1iAAAARGp1bWRjYm9yABEAEIAAAKoAOJtxE2MycGEuaW5ncmVkaWVudC52MwAAAAAYYzJzaD3+K8JY31AKOfwzFoYbyd0AAABwY2JvcqNpZGM6Zm9ybWF0bWltYWdlL3N2Zyt4bWxqaW5zdGFuY2VJRHgseG1wOmlpZDozYmViYWM3NC1jZjcyLTQ1NWUtOGI5MS02NDM1MGZlZjA3YTdscmVsYXRpb25zaGlwaHBhcmVudE9mAAAB4mp1bWIAAABBanVtZGNib3IAEQAQgAAAqgA4m3ETYzJwYS5hY3Rpb25zLnYyAAAAABhjMnNoJYwq31fmFCOOMKpSPBQfNgAAAZljYm9yomdhY3Rpb25zgqJmYWN0aW9ua2MycGEub3BlbmVkanBhcmFtZXRlcnOha2luZ3JlZGllbnRzgaJjdXJseC1zZWxmI2p1bWJmPWMycGEuYXNzZXJ0aW9ucy9jMnBhLmluZ3JlZGllbnQudjNkaGFzaFggZkPUu0GisN+ce+FryeshHz0KVbR3r0K0aNpVig7gVaGkZmFjdGlvbngdY29tLmFudGhyb3BpYy5jbGF1ZGUucHJvdmlkZWRqcGFyYW1ldGVyc6F4H2NvbS5hbnRocm9waWMub3JpZ2luLWNvbmZpZGVuY2VndW5rbm93bmtkZXNjcmlwdGlvbnhmQ2xhdWRlIHByb3ZpZGVkIHRoaXMgZmlsZSBhdCB0aGUgcmVxdWVzdCBvZiBhIHVzZXIgYW5kIG1heSBoYXZlIGNyZWF0ZWQgb3IgbW9kaWZpZWQgdGhlIGZpbGUgY29udGVudHMubXNvZnR3YXJlQWdlbnShZG5hbWVmQ2xhdWRlcmFsbEFjdGlvbnNJbmNsdWRlZPUAAADIanVtYgAAAEBqdW1kY2JvcgARABCAAACqADibcRNjMnBhLmhhc2guZGF0YQAAAAAYYzJzaPipaanGqTvos1hck8NF9dwAAACAY2JvcqVjYWxnZnNoYTI1NmNwYWRMAAAAAAAAAAAAAAAAZGhhc2hYIIr7OmSf78PIRrWGu3FNRUt1D+XhNfCFGHVX3Cqqmd9iZG5hbWVuanVtYmYgbWFuaWZlc3RqZXhjbHVzaW9uc4GiZXN0YXJ0GQE1Zmxlbmd0aBkeBAAAAj5qdW1iAAAAJ2p1bWRjMmNsABEAEIAAAKoAOJtxA2MycGEuY2xhaW0udjIAAAACD2Nib3KlY2FsZ2ZzaGEyNTZpc2lnbmF0dXJleE1zZWxmI2p1bWJmPS9jMnBhL3VybjpjMnBhOmI1N2NkZWQ0LTY2M2EtNGE5MC1hY2UyLWRlYmNkMjhkOGFmZi9jMnBhLnNpZ25hdHVyZWppbnN0YW5jZUlEeCx4bXA6aWlkOmM4YjE3NTUzLWJkNjAtNGY0OS1iMmQyLWIxMzQ0ZDI5MmE2Y3JjcmVhdGVkX2Fzc2VydGlvbnODomN1cmx4LXNlbGYjanVtYmY9YzJwYS5hc3NlcnRpb25zL2MycGEuaW5ncmVkaWVudC52M2RoYXNoWCBmQ9S7QaKw35x74WvJ6yEfPQpVtHevQrRo2lWKDuBVoaJjdXJseCpzZWxmI2p1bWJmPWMycGEuYXNzZXJ0aW9ucy9jMnBhLmFjdGlvbnMudjJkaGFzaFggNzV/fhCdX/mg1GVMIjY4gdp7ex9WNHPwshFUFMwhl+miY3VybHgpc2VsZiNqdW1iZj1jMnBhLmFzc2VydGlvbnMvYzJwYS5oYXNoLmRhdGFkaGFzaFggR8gvi9I8N6OeroIJ2mJJxvoIrxdt0c/ya9ukjKyTfsF0Y2xhaW1fZ2VuZXJhdG9yX2luZm+jZG5hbWVvQW50aHJvcGljIEZpbGVzZ3ZlcnNpb25lMS4wLjBrc3BlY1ZlcnNpb25lMi40LjAAABA4anVtYgAAAChqdW1kYzJjcwARABCAAACqADibcQNjMnBhLnNpZ25hdHVyZQAAABAIY2JvctKEWQISogEmGCFZAgowggIGMIIBjaADAgECAhRA5aAK7sI50L64g/oGQgU9Z1UTADAKBggqhkjOPQQDAzBJMRcwFQYDVQQKEw5BbnRocm9waWMsIFBCQzEuMCwGA1UEAxMlQW50aHJvcGljIENvbnRlbnQgQ3JlZGVudGlhbHMgUm9vdCBDQTAeFw0yNjA4MDcxODQzNTZaFw0yODA4MDYxOTQzNTZaMEQxFzAVBgNVBAoTDkFudGhyb3BpYywgUEJDMSkwJwYDVQQDEyBBbnRocm9waWMgQ2xhdWRlIENvbnRlbnQgU2lnbmluZzBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABJh6CmvLUBgFFNU0vUKlOVtE6djd17L5SuwX0LemFisBM3dkd/3cyjxFA3Qo5S46fX0/ihY0VZ7mfb9KF703t5OjWDBWMA4GA1UdDwEB/wQEAwIHgDAVBgNVHSUEDjAMBgorBgEEAYPoXgIBMAwGA1UdEwEB/wQCMAAwHwYDVR0jBBgwFoAUzlHiBIFOZFsj+OPEz5o+nMHXXMIwCgYIKoZIzj0EAwMDZwAwZAIwMXMdFJ4BetLLVY7ORuE9noqbbAZOZn/aArXyTwFAZfKrPzxF2vPoJNf1+UCdg1XGAjBwX1zd9WGqYkqmL5SFqw1QySjr1zJfpJM9+1rdDwSPLMOPOjKuiXjoU/pUUeG9RwmhY3BhZFkNngAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPZYQNL17Q7L5kqvEzuwI6jWek1FhcoXA1U0dL7ApUzYNdovH5V4tgtBk6BEDFK2yxGXj+9z8dAXpjKNyx6i5GJXKKI=</c2pa:manifest></metadata>\n\n  <g id=\"bird\" transform=\"translate(-300,-300)\">\n    <g id=\"bird-inner\" transform=\"translate(-125,-202)\">\n\n      \n      <g id=\"wing-back\">\n        <path fill=\"#682335\" d=\"M182.966,166.346c-31.021-83.869-32.794-128.997-32.794-128.997s-17.563,6.947-15.411,29.655c0,0-12.349-3.175-10.621,13.499c0,0-12.383,4.284-7.232,20.091c0,0-11.543,5.34-3.566,19.694c0,0-6.411,8.952,2.124,20.34C115.466,140.628,129.95,175.549,182.966,166.346z\"></path>\n      </g>\n\n      \n      <g id=\"tail\">\n        <path fill=\"#EF3F1D\" d=\"M101.015,142.396l-80.741,13.16c0,0,17.608,22.522,35.977,16.924c18.369-5.599,50.282-24.441,50.282-24.441L101.015,142.396z\"></path>\n        <path fill=\"#FF684A\" d=\"M103.573,144.424l-67.166,21.24c0,0,17.8,17.005,32.758,9.953c14.957-7.052,39.811-27.07,39.811-27.07L103.573,144.424z\"></path>\n      </g>\n\n      \n      <g id=\"legs\">\n        <path fill=\"#F2AC30\" d=\"M130.593,190.746c-0.193,0-0.39-0.033-0.582-0.104c-14.485-5.329-23.324-13.16-21.495-19.042c1.654-5.319,11.126-6.191,25.988-2.391c0.902,0.23,1.446,1.148,1.215,2.049c-0.23,0.901-1.15,1.444-2.049,1.215c-16.045-4.101-21.364-1.711-21.936,0.128c-0.904,2.907,5.35,9.695,19.441,14.879c0.873,0.321,1.321,1.29,0.999,2.163C131.923,190.324,131.278,190.746,130.593,190.746z\"></path>\n        <path fill=\"#BF6E1D\" d=\"M134.399,186.752c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.098,2.058c1.704,1.978,2.036,4.831,1.129,7.035c-0.856,2.251-2.847,3.88-4.924,4.387c-2.085,0.548-4.14,0.213-5.888-0.539c1.904-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.357-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C133.047,187.395,133.667,186.836,134.399,186.752z\"></path>\n        <path fill=\"#DD8924\" d=\"M130.593,186.565c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.098,2.058c1.704,1.978,2.036,4.831,1.129,7.035c-0.856,2.252-2.847,3.88-4.924,4.387c-2.085,0.548-4.14,0.213-5.888-0.539c1.905-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.358-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C129.24,187.208,129.861,186.649,130.593,186.565z\"></path>\n        <path fill=\"#F2AC30\" d=\"M128.159,186.565c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.098,2.058c1.704,1.978,2.036,4.831,1.129,7.035c-0.856,2.252-2.847,3.88-4.924,4.387c-2.085,0.548-4.14,0.213-5.888-0.539c1.905-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.358-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C126.807,187.208,127.427,186.649,128.159,186.565z\"></path>\n        <path fill=\"#F2AC30\" d=\"M114.37,192.992c-0.193,0-0.39-0.033-0.582-0.104c-14.485-5.329-23.324-13.16-21.495-19.042c1.654-5.319,11.127-6.192,25.988-2.391c0.902,0.23,1.446,1.148,1.215,2.049c-0.23,0.902-1.15,1.444-2.049,1.215c-16.046-4.102-21.364-1.71-21.936,0.128c-0.904,2.907,5.35,9.695,19.441,14.879c0.873,0.321,1.321,1.289,0.999,2.163C115.7,192.57,115.055,192.992,114.37,192.992z\"></path>\n        <path fill=\"#BF6E1D\" d=\"M118.176,188.998c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.098,2.058c1.704,1.978,2.036,4.831,1.129,7.035c-0.856,2.252-2.846,3.88-4.924,4.387c-2.085,0.548-4.14,0.214-5.888-0.539c1.905-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.357-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C116.824,189.641,117.444,189.083,118.176,188.998z\"></path>\n        <path fill=\"#DD8924\" d=\"M114.37,188.811c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.098,2.058c1.704,1.978,2.036,4.83,1.129,7.035c-0.856,2.251-2.847,3.88-4.924,4.387c-2.085,0.548-4.14,0.214-5.888-0.539c1.905-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.357-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C113.017,189.454,113.638,188.895,114.37,188.811z\"></path>\n        <path fill=\"#F2AC30\" d=\"M111.936,188.811c1.09-0.125,2.254-0.025,3.404,0.349c1.145,0.378,2.257,1.081,3.097,2.058c1.704,1.978,2.036,4.83,1.129,7.035c-0.856,2.251-2.846,3.88-4.924,4.387c-2.085,0.548-4.14,0.214-5.888-0.539c1.905-0.177,3.701-0.574,5.08-1.437c1.379-0.848,2.249-2.114,2.497-3.43c0.256-1.319-0.115-2.604-0.871-3.44c-0.756-0.855-1.978-1.357-3.523-1.614l-0.085-0.014c-0.923-0.153-1.547-1.026-1.394-1.949C110.584,189.454,111.204,188.895,111.936,188.811z\"></path>\n      </g>\n\n      \n      <path fill=\"#EF3F1D\" d=\"M205.762,153.804c0,16.874-20.198,34.446-43.035,34.446c-51.477,0-69.697-30.886-70.258-41.743c-0.872-16.851,62.245-26.581,85.171-26.581C200.566,119.925,205.762,136.93,205.762,153.804z\"></path>\n      <path fill=\"#FFFDF8\" d=\"M190.825,122.442c2.199,5.517,2.957,11.958,2.957,18.384c0,16.874-20.198,34.446-43.035,34.446c-25.059,0-42.235-7.32-53.331-16.085c8.392,13.025,28.216,29.063,65.311,29.063c22.837,0,43.035-17.572,43.035-34.446C205.762,140.825,202.678,127.778,190.825,122.442z\"></path>\n\n      \n      <g id=\"head\">\n        <path fill=\"#D17D19\" d=\"M204.406,157.193c1.613,4.123,17.478-2.51,34.507-2.51h-32.984L204.406,157.193z\"></path>\n        <path fill=\"#FFCD40\" d=\"M206.646,139.267c27.068,0,31.55,8.246,32.267,15.416h-31.908L206.646,139.267z\"></path>\n        <path fill=\"#ED9B2F\" d=\"M209.429,139.3c-0.902-0.02-1.825-0.033-2.783-0.033l0.359,15.416h7.423c0.005-0.209,0.016-0.417,0.016-0.628C214.444,148.526,212.579,143.42,209.429,139.3z\"></path>\n        <path fill=\"#252538\" d=\"M231.277,143.486c-2.771,2.2-4.554,5.592-4.554,9.405c0,0.611,0.06,1.206,0.148,1.793h12.041C238.521,150.764,236.994,146.526,231.277,143.486z\"></path>\n        <path fill=\"#FFDC83\" d=\"M226.324,143.493c-0.105,0.802-2.598,1.137-5.567,0.748c-2.969-0.389-5.291-1.355-5.186-2.157c0.105-0.802,2.598-1.137,5.567-0.748C224.107,141.725,226.429,142.691,226.324,143.493z\"></path>\n        <path fill=\"#EF3F1D\" d=\"M196.863,98.566L196.863,98.566c-5.125,9.443-3.523,23.635,3.578,31.698l0,0C205.566,120.821,203.964,106.63,196.863,98.566z\"></path>\n        <path fill=\"#FF684A\" d=\"M184.391,102.609L184.391,102.609c-0.946,10.702,6.168,23.086,15.89,27.66h0C201.227,119.567,194.113,107.183,184.391,102.609z\"></path>\n        <ellipse fill=\"#EF3F1D\" cx=\"181.44\" cy=\"148.796\" rx=\"28.797\" ry=\"28.035\"></ellipse>\n        <path fill=\"#A31332\" d=\"M167.185,120.391c0.529,7.289,22.885,10.482,32.865,9.79c-4.193-6.159-11.165-10.255-22.41-10.255C174.731,119.925,171.174,120.083,167.185,120.391z\"></path>\n        <ellipse fill=\"#A31332\" cx=\"187.356\" cy=\"151.041\" rx=\"19.817\" ry=\"17.513\"></ellipse>\n        <ellipse fill=\"#FFFDF8\" cx=\"188.604\" cy=\"148.982\" rx=\"19.04\" ry=\"16.827\"></ellipse>\n        <path fill=\"#CCE2EA\" d=\"M201.524,148.253h-16.915c0.003,0.081-0.001,0.161,0.004,0.242c0.317,4.667,4.356,8.193,9.023,7.876C197.998,156.076,201.362,152.527,201.524,148.253z\"></path>\n        <circle id=\"pupil\" fill=\"#252538\" cx=\"192.7\" cy=\"148\" r=\"5.1\"></circle>\n        <ellipse id=\"lid\" fill=\"#EF3F1D\" cx=\"188.604\" cy=\"148.982\" rx=\"19.6\" ry=\"17.4\" transform=\"matrix(1 0 0 0 0 148.982)\"></ellipse>\n      </g>\n\n      \n      <g id=\"wing-front\">\n        <path fill=\"#A31332\" d=\"M167.032,163.354C111.874,92.97,96.399,50.541,96.399,50.541s-14.601,11.98-5.616,32.945c0,0-12.729,0.749-5.99,16.098c0,0-10.483,7.862-0.749,21.34c0,0-9.359,8.611,2.621,19.842c0,0-3.369,10.483,8.236,18.719C94.902,159.485,119.361,188.312,167.032,163.354z\"></path>\n        <path fill=\"#682335\" d=\"M167.032,163.354c-9.831-12.544-18.395-24.195-25.841-34.881c-25.344-16.081-16.773,3.477-12.533,7.365c0,0-16.098,6.739,5.428,17.408c0,0-12.916,3.931-3.744,9.734c0,0-6.907,8.104,5.019,10.321C144.424,172.53,154.986,169.661,167.032,163.354z\"></path>\n      </g>\n    </g>\n  </g>\n\n  \n\n</svg>";

  var DEFAULTS = {
    scale: 0.42,   // taille de l'oiseau
    near: 230,     // px : en deca de cette distance il reste assis et suit du regard
    idle: 550,     // ms d'immobilite du curseur avant l'atterrissage
    stiff: 0.03,   // nervosite du vol (plus bas = poursuite plus paresseuse)
    damp: 0.7,     // amortissement : 0.7 ~ critique, aucun rebond ressort
    lag: 0.035,    // temps de reaction : l'oiseau vise une position retardee du curseur
    gaze: 340,     // px : portee du regard
    standoff: 150, // px : distance minimale gardee avec le curseur (il ne colle jamais au curseur)
    zIndex: 9999
  };

  var rafId = null, rootEl = null, bubEl = null, listeners = [];

  function destroy() {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = null;
    listeners.forEach(function (l) { l[0].removeEventListener(l[1], l[2]); });
    listeners = [];
    if (rootEl && rootEl.parentNode) rootEl.parentNode.removeChild(rootEl);
    if (bubEl && bubEl.parentNode) bubEl.parentNode.removeChild(bubEl);
    rootEl = null; bubEl = null;
  }

  function config(opts) {
    var cfg = {}, k;
    for (k in DEFAULTS) cfg[k] = DEFAULTS[k];
    var tag = document.currentScript || document.querySelector('script[src*="bird-cursor"]');
    if (tag) for (k in DEFAULTS) {
      var a = tag.getAttribute('data-' + k.toLowerCase());
      if (a !== null) { var v = parseFloat(a); if (!isNaN(v)) cfg[k] = v; }
    }
    if (opts) for (k in opts) if (opts[k] !== undefined) cfg[k] = opts[k];
    return cfg;
  }

  function mount(opts) {
    destroy();
    var cfg = config(opts);
    if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    Array.prototype.forEach.call(document.querySelectorAll('#bird-follow, #bird-follow-bubble'), function (n) { n.remove(); });

    var wrap = document.createElement('div');
    wrap.innerHTML = MARKUP;
    var root = wrap.firstElementChild;
    root.style.zIndex = cfg.zIndex;
    document.body.appendChild(root);
    rootEl = root;

    function $(id) { return root.querySelector('#' + id); }
    var bird = $('bird'), head = $('head'), pupil = $('pupil'), lid = $('lid'),
        wf = $('wing-front'), wb = $('wing-back'), legs = $('legs'), tail = $('tail');

    function on(target, type, fn, o) { target.addEventListener(type, fn, o); listeners.push([target, type, fn]); }

    var W = 0, H = 0;
    function resize() {
      W = document.documentElement.clientWidth; H = document.documentElement.clientHeight;
      root.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
    }
    resize();
    on(window, 'resize', resize);

    var mx = W * 0.5, my = H * 0.4, lastMove = -9999;
    function onMove(e) {
      var p = e.touches ? e.touches[0] : e;
      mx = p.clientX; my = p.clientY; lastMove = performance.now();
    }
    on(document, 'mousemove', onMove, { passive: true });
    on(document, 'touchmove', onMove, { passive: true });

    // ---- bulle de conversation
    var bub = document.createElement('div');
    bub.id = 'bird-follow-bubble';
    bub.style.cssText = 'position:fixed;left:0;top:0;pointer-events:none;z-index:' + (cfg.zIndex + 1) + ';will-change:transform';
    var bubIn = document.createElement('div');
    bubIn.style.cssText = 'transform-origin:50% 100%;opacity:0;transform:translate(-50%,-100%) scale(.8);' +
      'transition:opacity .2s ease, transform .28s cubic-bezier(.2,1.5,.4,1)';
    var bubBox = document.createElement('div');
    bubBox.setAttribute('role', 'status');
    bubBox.style.cssText = 'position:relative;max-width:264px;background:#FFFDF8;color:#682335;' +
      'border:2px solid #682335;border-radius:15px;padding:11px 15px;' +
      "font:500 14px/1.45 Karla,system-ui,-apple-system,sans-serif;text-wrap:pretty;" +
      'box-shadow:0 5px 0 rgba(104,35,53,.16)';
    var bubText = document.createElement('span');
    var bubTail = document.createElement('div');
    bubTail.style.cssText = 'position:absolute;left:50%;bottom:-8px;width:13px;height:13px;margin-left:-6.5px;' +
      'background:#FFFDF8;border-right:2px solid #682335;border-bottom:2px solid #682335;' +
      'transform:rotate(45deg);border-bottom-right-radius:3px';
    bubBox.appendChild(bubText); bubBox.appendChild(bubTail);
    bubIn.appendChild(bubBox); bub.appendChild(bubIn);
    document.body.appendChild(bub);
    bubEl = bub;

    var sayEl = null, sayText = '', shownText = '', sayW = 0;
    var sayPend = null, sayPendAt = 0;   // cible survolee en attente de confirmation
    function findSay(t) { return (t && t.closest) ? t.closest('[data-bird-say]') : null; }
    on(document, 'mouseover', function (e) {
      var el = findSay(e.target);
      if (el && el !== sayEl) { sayPend = el; sayPendAt = performance.now(); }
    }, { passive: true });
    on(document, 'mouseout', function (e) {
      var el = findSay(e.target);
      if (el && !(e.relatedTarget && el.contains(e.relatedTarget))) {
        if (el === sayPend) sayPend = null;
        if (el === sayEl) { sayEl = null; sayText = ''; }
      }
    }, { passive: true });

    var s = cfg.scale;
    var x = mx, y = my, vx = 0, vy = 0;
    var mode = 'land', face = 1, faceA = 1, turn = 0, tilt = 0, flapPh = 0, boost = 0;
    var wingBase = -54, wingAmp = 3, legLift = 0, legOp = 1;
    var gx = 0, gy = 0, headA = 0, lidK = 0, blinkAt = 0, blinkEnd = 0;
    var perch = null, lagX = mx, lagY = my, ltx = mx, lty = my, hasT = false;
    var t0 = performance.now(), prev = t0;

    function clamp(v, a, b) { return v < a ? a : (v > b ? b : v); }
    function lerp(a, b, k) { return a + (b - a) * k; }

    // --- point d'appui sur le bord haut d'un composant, decale d'au moins cfg.standoff du curseur
    function edgeX(r) {
      var lo = r.left + 20, hi = Math.max(lo, r.right - 20);
      var away = mx + (mx - (r.left + r.width / 2) >= 0 ? 1 : -1) * cfg.standoff;
      var px = clamp(away, lo, hi);
      if (Math.abs(px - mx) < cfg.standoff * 0.75) {
        var alt = clamp(mx - (away - mx), lo, hi);
        if (Math.abs(alt - mx) > Math.abs(px - mx)) px = alt;
      }
      return px;
    }

    // --- choisit le composant [data-bird-perch] le plus proche du curseur
    function pickPerch() {
      var els = document.querySelectorAll('[data-bird-perch]'), best = null, bd = 1e9;
      for (var i = 0; i < els.length; i++) {
        var r = els[i].getBoundingClientRect();
        if (r.width < 44 || r.bottom < 0 || r.top > H) continue;
        var px = edgeX(r);
        var d = Math.hypot(mx - px, my - r.top);
        if (d < bd) { bd = d; best = { el: els[i], fx: (px - r.left) / r.width }; }
      }
      return best;
    }
    // le point d'appui est recalcule a chaque frame : l'oiseau reste collé au composant au scroll
    function perchPoint(p) {
      var fb = { x: clamp(mx + cfg.standoff, 60, Math.max(70, W - 60)), y: H - 4 };
      if (!p) return fb;
      var r = p.el.getBoundingClientRect();
      if (r.width < 2) return fb;
      return { x: p.hold ? r.left + p.fx * r.width : edgeX(r), y: r.top + 2 };
    }

    function frame(now) {
      var dt = clamp((now - prev) / 16.67, 0.2, 3); prev = now;
      var t = (now - t0) / 1000;

      lagX = lerp(lagX, mx, cfg.lag * dt);   // le curseur "vu" par l'oiseau, avec un temps de retard
      lagY = lerp(lagY, my, cfg.lag * dt);

      var hx = x + face * 45 * s, hy = y - 52 * s;          // position de la tete a l'ecran
      var dist = Math.hypot(mx - hx, my - hy);
      var idle = (now - lastMove) > cfg.idle;

      // ---- survol confirme apres 130 ms : evite de sauter d'une carte a l'autre
      if (sayPend && now - sayPendAt > 130) {
        sayEl = sayPend; sayText = sayEl.getAttribute('data-bird-say') || ''; sayPend = null;
      }

      // ---- etats : chase (poursuite) -> land (atterrissage) -> perch (posé)
      if (sayEl) {
        if (!perch || perch.el !== sayEl) { perch = { el: sayEl }; if (mode === 'perch') mode = 'land'; }
      } else if (perch && perch.say) { perch = pickPerch(); if (mode === 'perch') mode = 'land'; }
      if (perch) perch.say = !!sayEl;

      if (sayEl) {
        if (mode === 'chase') mode = 'land';
        if (mode === 'land' && perch) perch.hold = false;
      } else if (mode === 'chase') {
        if (idle) { mode = 'land'; perch = pickPerch(); }
      } else if (!idle && dist > cfg.near) {
        if (mode === 'perch') { vy -= 7; vx += face * 1.5; boost = 1; }   // decollage
        mode = 'chase';
      }

      var tx, ty;
      if (mode === 'chase') {
        // point situe a cfg.standoff du curseur, du cote ou l'oiseau se trouve deja, legerement en hauteur
        var ax = x - lagX, ay = (y - lagY) - cfg.standoff * 0.32, an = Math.hypot(ax, ay);
        if (an < 1) { ax = -1; ay = -0.4; an = 1.08; }
        tx = lagX + (ax / an) * cfg.standoff;
        ty = lagY + (ay / an) * cfg.standoff + 50 * s;
      } else {
        if (!perch) perch = pickPerch();
        var pp = perchPoint(perch); tx = pp.x; ty = pp.y;
      }

      // lissage de la cible : l'oiseau ne recoit jamais un changement de direction sec
      var rtx = tx, rty = ty;                  // cible reelle, non lissee
      if (!hasT) { ltx = tx; lty = ty; hasT = true; }
      var tk = clamp((mode === 'chase' ? 0.12 : 0.055) * dt, 0, 0.5);
      ltx = lerp(ltx, tx, tk); lty = lerp(lty, ty, tk);
      if (mode !== 'perch') { tx = ltx; ty = lty; }
      else { ltx = tx; lty = ty; }

      if (mode === 'perch') {
        var pp2 = perchPoint(perch); x = pp2.x; y = pp2.y; vx = 0; vy = 0;
      } else if (mode === 'land') {
        // approche exponentielle : il ralentit et se pose, sans jamais depasser la cible
        var ap = clamp(0.075 * dt, 0, 0.5);
        var ex = x + (tx - x) * ap, ey = y + (ty - y) * ap;
        var stepMax = 13 * dt, stepN = Math.hypot(ex - x, ey - y);
        if (stepN > stepMax) { ex = x + (ex - x) * stepMax / stepN; ey = y + (ey - y) * stepMax / stepN; }
        vx = (ex - x) / Math.max(dt, 0.2); vy = (ey - y) / Math.max(dt, 0.2);
        x = ex; y = ey;
        if (Math.hypot(rtx - x, rty - y) < 2.5) {
          mode = 'perch'; x = tx; y = ty; vx = 0; vy = 0;
          if (perch && perch.el) {                      // fige la position sur le composant
            var rl = perch.el.getBoundingClientRect();
            if (rl.width > 2) { perch.hold = true; perch.fx = (x - rl.left) / rl.width; }
          }
        }
      } else {
        vx += (tx - x) * cfg.stiff * dt; vy += (ty - y) * cfg.stiff * dt;
        vx *= Math.pow(cfg.damp, dt); vy *= Math.pow(cfg.damp, dt);
        x += vx * dt; y += vy * dt;
        x = clamp(x, 30, Math.max(40, W - 30)); y = clamp(y, 80, Math.max(90, H - 2));
      }

      // ---- orientation
      var wantFace = (mx - x) > 26 ? 1 : ((mx - x) < -26 ? -1 : face);
      if (wantFace !== face) { face = wantFace; turn = 1; }          // demi-tour : petit saut + battement
      faceA = lerp(faceA, face, 0.12 * dt);
      if (Math.abs(faceA - face) < 0.02) faceA = face;
      turn = Math.max(0, turn - 0.042 * dt);

      // ---- ailes, inclinaison, pattes
      var speed = Math.hypot(vx, vy);
      boost = Math.max(0, boost - 0.04 * dt);
      var flying = mode !== 'perch';
      var baseT = flying ? (mode === 'land' ? 12 : 2) : -54;
      var ampT = flying ? (mode === 'land' ? 30 : 34 + boost * 14) : 3;
      wingBase = lerp(wingBase, baseT, 0.12 * dt);
      wingAmp = lerp(wingAmp, ampT, 0.14 * dt);
      var flapSpd = flying ? (10 + Math.min(speed, 14) * 0.45 + boost * 6) : 1.6;
      flapPh += flapSpd * 0.0167 * dt;
      var flap = Math.sin(flapPh * 6.283) * (wingAmp + turn * 20);

      var tiltT = flying ? clamp(vy * 0.7, -20, 22) - clamp(Math.abs(vx) * 0.25, 0, 8) : 0;
      tilt = lerp(tilt, tiltT, 0.12 * dt);

      legLift = lerp(legLift, flying ? -12 : 0, 0.12 * dt);
      legOp = lerp(legOp, flying ? 0.25 : 1, 0.12 * dt);

      // ---- regard : tete + pupille vers le curseur
      var seen = dist < cfg.gaze;
      var ldx = (mx - hx) * face, ldy = (my - hy);
      var headT = seen ? clamp(Math.atan2(ldy, Math.max(Math.abs(ldx), 26)) * 57.3 * 0.95, -38, 42) : Math.sin(t * 0.7) * 6;
      headA = lerp(headA, headT, 0.13 * dt);
      var gxT = seen ? clamp(ldx * 0.045, -5.5, 6.5) : Math.sin(t * 0.7) * 3;
      var gyT = seen ? clamp(ldy * 0.035, -5, 5.5) : Math.sin(t * 0.45) * 1.5;
      gx = lerp(gx, gxT, 0.16 * dt); gy = lerp(gy, gyT, 0.16 * dt);

      // ---- clignement
      if (now > blinkAt) { blinkAt = now + 2200 + Math.random() * 4200; blinkEnd = now + 130; }
      lidK = now < blinkEnd ? Math.sin(((blinkEnd - now) / 130) * 3.1416) : 0;

      // ---- rendu
      var bob = (mode === 'perch' ? Math.sin(t * 1.5) * 0.6 : 0) - Math.sin((1 - turn) * 3.1416) * 4;
      bird.setAttribute('transform', 'translate(' + x.toFixed(2) + ',' + (y + bob).toFixed(2) +
        ') scale(' + (faceA * s).toFixed(4) + ',' + s.toFixed(4) + ') rotate(' + (tilt - turn * 4).toFixed(2) + ',25,-47)');
      wf.setAttribute('transform', 'rotate(' + (wingBase + flap).toFixed(2) + ',168,163)');
      wb.setAttribute('transform', 'rotate(' + (wingBase * 1.28 - flap * 0.85).toFixed(2) + ',183,167)');
      legs.setAttribute('transform', 'translate(0,' + legLift.toFixed(2) + ')');
      legs.setAttribute('opacity', legOp.toFixed(3));
      tail.setAttribute('transform', 'rotate(' + (mode === 'perch' ? 0 : clamp(-vy * 0.5, -8, 8)).toFixed(2) + ',108,148)');
      head.setAttribute('transform', 'rotate(' + headA.toFixed(2) + ',172,152)');
      pupil.setAttribute('cx', (192.7 + gx).toFixed(2));
      pupil.setAttribute('cy', (148 + gy).toFixed(2));
      lid.setAttribute('transform', 'matrix(1 0 0 ' + lidK.toFixed(3) + ' 0 ' + (148.982 * (1 - lidK)).toFixed(2) + ')');

      // ---- bulle
      if (sayText) {
        if (sayText !== shownText) { bubText.textContent = sayText; shownText = sayText; sayW = 0; }
        if (!sayW) sayW = bubBox.offsetWidth;
        var bx = clamp(x + faceA * 30 * s, sayW / 2 + 10, Math.max(sayW / 2 + 12, W - sayW / 2 - 10));
        var by = Math.max(bubBox.offsetHeight + 14, y - 122 * s);
        bub.style.transform = 'translate(' + bx.toFixed(1) + 'px,' + by.toFixed(1) + 'px)';
        bubIn.style.opacity = '1';
        bubIn.style.transform = 'translate(-50%,-100%) scale(1)';
      } else if (shownText) {
        bubIn.style.opacity = '0';
        bubIn.style.transform = 'translate(-50%,-100%) scale(.8)';
        shownText = '';
      }

      rafId = requestAnimationFrame(frame);
    }
    frame(performance.now());   // premiere pose rendue immediatement
  }

  window.BirdCursor = { mount: mount, destroy: destroy, defaults: DEFAULTS };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', function () { mount(); });
  else mount();
})();
