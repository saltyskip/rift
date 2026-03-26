(function() {
  var DEFAULT_BASE = "https://api.riftl.ink";
  var _publishableKey = null;

  var Rift = {
    init: function(publishableKey, opts) {
      opts = opts || {};
      _publishableKey = publishableKey;
      if (opts.baseUrl) DEFAULT_BASE = opts.baseUrl;
    },

    // Fire-and-forget click tracking. Use on <a> tag onClick handlers.
    // The <a> tag handles navigation — Universal Links work normally.
    click: function(linkId) {
      if (!_publishableKey) {
        console.warn("Rift: call Rift.init('pk_live_...') before Rift.click()");
        return;
      }

      var url = DEFAULT_BASE + "/v1/attribution/click?key=" + encodeURIComponent(_publishableKey);
      var blob = new Blob(
        [JSON.stringify({ link_id: linkId })],
        { type: "application/json" }
      );

      if (navigator.sendBeacon) {
        navigator.sendBeacon(url, blob);
      } else {
        fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ link_id: linkId }),
          keepalive: true
        }).catch(function(){});
      }
    },

    getLink: function(linkId, opts) {
      opts = opts || {};
      var base = opts.baseUrl || DEFAULT_BASE;
      return fetch(base + "/r/" + encodeURIComponent(linkId), {
        headers: { "Accept": "application/json" }
      }).then(function(r) { return r.json(); });
    }
  };

  if (typeof window !== "undefined") window.Rift = Rift;
  if (typeof module !== "undefined") module.exports = Rift;
})();
