//
// DeferredDeepLink.swift
// RiftSDK
//
// Hand-written iOS convenience for deferred deep linking. Not generated —
// do not overwrite.
//
// `RiftSdk.checkDeferredDeepLink(clipboardText:)` (generated) deliberately
// takes the clipboard text as an argument rather than reading the pasteboard
// itself, because on iOS 16+ reading `UIPasteboard.general.string` surfaces
// the "pasted from <app>" disclosure banner — and the SDK shouldn't decide
// when that appears.
//
// This wrapper makes the common case safe: it uses
// `UIPasteboard.detectPatterns(for:)` to check whether the pasteboard contains
// a probable web URL *without* accessing the contents and *without* triggering
// the disclosure. Only when a URL is actually present does it read the
// pasteboard (the one access that may show the banner) and hand the text to
// the core, which validates the host before attributing.
//
// Net effect: apps no longer read the clipboard — or flash the paste banner —
// on launches where there's nothing to defer.
//

import Foundation

#if canImport(UIKit)
import UIKit

public extension RiftSdk {
    /// Deferred deep linking that only touches the system pasteboard when it
    /// actually contains a URL.
    ///
    /// Call once, early in your post-install / first-launch flow. Returns the
    /// resolved link (and reports attribution) when the pasteboard holds a
    /// trusted Rift URL; returns `nil` — without reading the pasteboard or
    /// showing the iOS paste banner — when it does not.
    ///
    /// On a successful match the pasteboard is cleared, so a re-launch doesn't
    /// re-attribute the same install. Pass `clearOnMatch: false` to leave it
    /// intact. Only the Rift URL we matched is cleared — unrelated clipboard
    /// contents never reach this point (host validation rejects them).
    func checkDeferredDeepLinkFromPasteboard(
        clearOnMatch: Bool = true
    ) async throws -> DeferredDeepLinkResult? {
        let pasteboard = UIPasteboard.general

        // Gate the read. `detectPatterns(for:)` neither exposes the contents
        // nor triggers the paste disclosure; it just reports which patterns are
        // present. If detection fails, skip rather than read blindly.
        let detected: Set<UIPasteboard.DetectedPatterns>
        do {
            detected = try await pasteboard.detectPatterns(for: [.probableWebURL])
        } catch {
            return nil
        }

        guard detected.contains(.probableWebURL) else {
            return nil
        }

        // A URL is present — read it (the only access that may surface the
        // banner) and let the core validate the host + report attribution.
        let clipboardText = pasteboard.string
        let result = try await checkDeferredDeepLink(clipboardText: clipboardText)

        if clearOnMatch, result != nil {
            pasteboard.string = ""
        }
        return result
    }
}
#endif
