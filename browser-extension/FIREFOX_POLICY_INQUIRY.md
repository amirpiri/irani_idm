# Mozilla AMO policy inquiry — draft

Use this text to ask Mozilla's Add-ons team for a policy determination before
submitting the Firefox package. This is a draft only; no inquiry has been sent.

**Subject:** Eligibility of a browser download-manager integration that hands URLs to a local desktop app

Hello Mozilla Add-ons team,

We are preparing a Firefox WebExtension that integrates browser downloads with
Irani IDM, a locally installed desktop download manager.

The add-on's behavior is narrowly limited to HTTP/HTTPS downloads. When the user
explicitly enables handoff, it detects a new browser download, cancels the
browser transfer, and passes that download's complete URL to the local Irani
IDM application through the operating system's registered `iranidm://` URL
protocol. The desktop application then downloads the file and manages pause,
resume, and multi-connection transfers.

The add-on does not promote or install the desktop application, does not load
remote code, does not access web-page contents, and does not use broad host
permissions or content scripts. Handoff is disabled by default. The user must
opt in on a first-run Options page. The manifest declares `browsingActivity`
and `websiteActivity` as optional Firefox data categories, requests them only
from the user's Options-page action, and disables handoff if either permission
is denied or revoked. The user can also decline and proceed to uninstall.

Could you confirm whether this browser-download-manager integration is
eligible for listing on AMO, particularly in light of the policy preview
scheduled to take effect on December 1, 2026? We would appreciate specific
guidance on both the restriction on add-ons whose sole purpose is launching an
outside application and the restriction on leaking local or user-specific
information to other applications. Is an explicitly opted-in download URL
handoff to the user's locally installed download manager considered permitted
functional data transmission, or is it prohibited? If it is not eligible, is
there a supported integration pattern that would preserve the core download-
manager handoff?

Thank you.

## Package details to provide with the inquiry

- Extension name: Irani IDM Download Integration
- Manifest version: 3
- Minimum Firefox: 140.0
- Extension ID: `irani-idm@irani-idm`
- Package: `store-packages/Irani-IDM-Firefox.zip`
- Privacy policy: `PRIVACY.md` (publish it to the public repository first)
- Reviewer test steps: `STORE_LISTING_FIREFOX.md`
- Policy preview: https://extensionworkshop.com/documentation/publish/add-on-policies-preview-2026-12/
