# Mecc meat collection

The normal partial Mecc controller calls the acquired PC proximity scan and
collection callback after each published movement step. No diagnostic switch
enables collection. Hunt a Vimp in Story2, let its meat settle, then approach the
pieces. The ordinary hint overlay displays the source-owned carried amount.

The scan uses a quarter-second global clock, ordered common-list candidates,
classes 1117–1119, their settled control bit and an XY distance strictly below
10 source units. It does not add a vertical-distance condition. The callback
refuses collection when the carried amount is already at least 20; a final piece
can take the total above 20. It retires the selected piece, adds its actual amount,
selects the source pickup cue and refreshes the carried-meat display. Collected
pieces leave drawing immediately and retire through the existing owned-remains
cleanup. Full inventory leaves pieces in the world to expire normally.

Collected retirement has its own callback witness. Expiry still requires the
actual completed fade; collection never sets that fade to simulate expiry.
Both paths require the real external-owner release and recheck references before
unlinking. The initial graphical candidate exposed the missing collected path;
its refused cleanup is retained privately and is not publication acceptance.

The source actor payload is the authority for carried meat. The hint text does
not maintain another inventory. A new mission entry constructs a fresh player
and collection scope; returning to the menu retires their existing owners.

## Prototype PS2 starting points

The lookup first verified the literal ELF names `mecc_collect_food` and
`mecc_check_food_chunk_collect` against the prototype identity pinned in
[RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md). Neither literal name
encodes a signature; no PS2 signature, ABI or memory layout is imported.

The PC addresses below refer only to the debug-1.1 executable with SHA256
`690bad39847b0f578163e547e6a6de10ffe9213635a069bd1ce96a75dfd39ead`.

The independently acquired PC callback, debug address `0x0047eeb0`, has converging
static evidence for `mecc_collect_food` at PS2 address `0x00166400`: the related
player selection/capacity branches and ordered mission-credit, item-retirement,
carried-resource, randomized pickup-cue and HUD-refresh operations. The
quarter-second PC primary block beginning at `0x0047d94f` corresponds in role to
`mecc_check_food_chunk_collect` at `0x00166528`, including class selection,
settlement, XY selection and the collection callback. This is a PC primary
**block**, not an independently established PC function entry.

Both relationships remain **probable**. The selected standard MIPS operations
were inspected, but a complete independent EE execution/control and the linked
and multiplayer variants remain unverified. PC and PS2 structure and call
boundaries differ. Previous PC aliases remain in private identity-bound evidence.
The public source search routes are in [SYMBOL_NAVIGATION.md](SYMBOL_NAVIGATION.md).
A name or attribution does not establish runtime fidelity.

`vimpchunk_destroy__FP6OBJECT` is another verified literal ELF search key for the
retirement investigation. Its exact relationship to the generic PC retirement
service remains **unknown**; it does not authorize a guessed state destructor.

## Mission admission boundary

The source-owned campaign subset is described centrally in
[MISSION_PROGRESSION.md](MISSION_PROGRESSION.md). An admitted ordinary Gameplay
record shares its actual collection quota and counter with its declared return
destination. Collection credit alone cannot complete that record: the current
player must also satisfy the acquired destination predicate. Inspection and
unsupported records keep the explicit no-objective-credit preview branch. Earlier
preview .11.4 always uses that uncredited branch; see the current download receipt
for delivered availability. Feeding, deposits, economy and replenishment remain
separate services in [BACKLOG.md](../BACKLOG.md).

The callback requests its actual local-player pickup cue through the common
sound transport. A queued request is not proof of a playing voice; graphical
acceptance must observe the real audio sink separately. The hint overlay is a
preview interface, not a claim of the original inventory HUD's visual fidelity.

The same graphical hunt exposed head-of-line blocking in the shared asynchronous
sound transport. Consecutive fixed-position one-shots with the same cue now stage
their independent voices together, in request and parameter-RNG order. Other
cues, tickets, actor/reuse requests and player controls remain barriers. Every
staged voice still needs its actual sink before the batch starts; the bounded
queue is not enlarged and no request is dropped or counted as played on staging.
This is a Bevy transport policy, not a claim of native audio scheduling/mix parity.
