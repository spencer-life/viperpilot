# Source provenance

This package is a GPL-2.0-only adaptation informed by:

- **ClickSync**, © its contributors, licensed GPL-2.0
- Repository: <https://github.com/Nuitfanee/ClickSync>
- Pinned revision: [`9c3ad10baa964c507ea0714e72bb9c47b3d591c2`](https://github.com/Nuitfanee/ClickSync/tree/9c3ad10baa964c507ea0714e72bb9c47b3d591c2)
- Primary source file: `src/protocols/protocol_api_razer.js`
- Connection-plan reference: `tools/test_razer_connection_plans.js`

Adapted facts include the Razer feature-report layout, XOR checksum, transaction
matching, command class/ID values, Viper V4 Pro VID/PIDs, DPI and polling
commands, onboard button-assignment encoding, modifier bits, HID keyboard
usages, side-button protocol IDs, and the wireless V2 two-report polling
sequence.

No claim is made that Razer endorses this utility. “Razer” and “Viper” are
trademarks of their respective owner.

Wireless timing was compared against **OpenRazer** revision
[`6820f9da169d354bc7e6e93a0aa8683a6bb75792`](https://github.com/openrazer/openrazer/tree/6820f9da169d354bc7e6e93a0aa8683a6bb75792),
primarily `driver/razerchromacommon.c` and `driver/razermouse_driver.c`.
OpenRazer does not list the Viper V4 Pro at that revision, so this is analogous
evidence only. Its longer inter-write delay failed when measured on this Viper
and is not used. ClickSync remains the Viper V4-specific protocol source.

ClickSync's selector order works reliably for 1000→4000 on this mouse but did
not reliably retain 4000→1000. A local reverse-selector experiment is clearly
identified as measured, device-specific evidence; it is not attributed to
either upstream project and is not retained in the production packet encoder.
