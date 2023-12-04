var cacheName = 'egui-template-pwa';
var filesToCache = [
  './',
  './index.html',
  './chord_finder.js',
  './chord_finder_bg.wasm',
];

/* Start the service worker and cache all of the app's content */
self.addEventListener('install', function (e) {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    })
  );
});

// TODO: figure out why this messes up hot reload
// /* Serve cached content when offline */
// self.addEventListener('fetch', function (e) {
//   console.log(e);
//   e.respondWith(
//     caches.match(e.request).then(function (response) {
//       return response || fetch(e.request);
//     })
//   );
// });
