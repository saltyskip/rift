(function() {
  var DEFAULT_BASE = "https://api.riftl.ink";
  var _publishableKey = null;
  var _domain = null;
  var _bound = false;

  var Rift = {
    init: function(publishableKey, opts) {
      opts = opts || {};
      _publishableKey = publishableKey;
      if (opts.baseUrl) DEFAULT_BASE = opts.baseUrl;
      if (opts.domain) _domain = opts.domain;

      // Auto-track clicks on links matching the custom domain.
      if (!_bound && _domain && typeof document !== "undefined") {
        _bound = true;
        document.addEventListener("click", function(e) {
          var a = e.target.closest("a[href]");
          if (!a) return;
          var prefix = "https://" + _domain + "/";
          var href = a.href;
          if (href.indexOf(prefix) === 0) {
            var linkId = href.slice(prefix.length).split("?")[0].split("#")[0];
            if (linkId) {
              Rift.click(linkId, { domain: _domain });
            }
          }
        }, true);
      }
    },

    // Fire-and-forget click tracking + clipboard write.
    // Called automatically for links matching the custom domain.
    // Can also be called manually for programmatic use cases.
    // Does NOT navigate — the <a> tag handles navigation so Universal Links work.
    //
    // opts.domain — custom domain for clipboard URL (e.g. "go.yourcompany.com").
    //               Defaults to the init domain or location.hostname.
    click: function(linkId, opts) {
      opts = opts || {};

      // Clipboard write — must happen here while we have the user gesture.
      if (typeof navigator !== "undefined" && navigator.clipboard) {
        var domain = opts.domain || _domain || (typeof location !== "undefined" ? location.hostname : null);
        if (domain) {
          var clipUrl = "https://" + domain + "/" + linkId;
          navigator.clipboard.writeText(clipUrl).catch(function(){});
        }
      }

      // Send click beacon.
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
