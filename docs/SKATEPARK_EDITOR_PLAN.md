# Skatepark editor for MW2 (2009)

## Goal

Build a Tony Hawk-style, in-game park editor on top of the existing Skate mode.
The first milestone is a local park: place objects, skate them immediately,
then save and reload the layout. The next milestone is a hosted multiplayer
park that friends can join through IW4L's existing room system. Park data can
be shared as small files without distributing either game's assets.

## Existing foundation

This runtime already loads MW2 maps and runs Skate physics against their
collision. `crates/render_anim/src/skate.rs` owns the Skate worker and collision
updates; `crates/render_anim/src/skate/collision.rs` converts MW2 collision to
Skate geometry. Minecraft mode shows that live geometry changes can trigger a
Skate collision rebuild.

## First playable milestone

1. Enter edit mode from a private match on any MW2 map. Pause Skate input.
2. Choose from a small palette: box, kicker, bank, quarter pipe, and rail.
3. Show a movable, rotatable ghost with a valid/invalid placement indicator.
4. Place, select, move, rotate, duplicate, and delete objects. Support undo and
   redo for each edit.
5. Rebuild the added objects' Skate collision when edits occur, including
   explicit rail paths for grinds. Keep the original map collision intact.
6. Exit edit mode and skate without reloading the map.
7. Save and load versioned JSON layouts tied to a map name. Reject invalid
   object types, non-finite coordinates, and layouts over the object limit.

## Multiplayer milestone

1. The host owns the park layout. Joining players receive a versioned layout
   snapshot before play and reliable edits thereafter. The host validates each
   edit and assigns the resulting revision; clients never apply an edit until
   the host accepts it.
2. All clients render the same obstacle geometry and build the same Skate
   collision from that layout. A new revision swaps collision off the render
   thread, as the existing Minecraft block collision path does.
3. Skate mode needs a networked state. Today its worker and presentation are
   local to one client, and its external motion changes only that client's
   simulation. Send a compact, bounded skater state to the match host; let the
   host validate motion and publish it in snapshots so peers can draw the
   correct position, board, and animation. Avoid using reliable edit messages
   for frame-by-frame motion.
4. Add join-in-progress, disconnect, respawn, map change, and incompatible
   client handling. A late joiner receives the current layout and player
   skate states before entering the match.
5. Test with two independent clients and a host on a real MW2 map. The host's
   room discovery uses IW4L's master/relay service. Release packaging includes
   code, licenses, and an install guide, never MW2 or Skate 3 files.

## Engineering sequence

1. **Park data and geometry:** Define versioned layout data, obstacle
   parameters, triangle/rail generation, validation, and round-trip tests.
2. **Skate collision integration:** Add park geometry to the worker's map
   collision. Rebuild off the render thread and swap the prepared collision on
   the Skate worker. Handle both ordinary MW2 maps and Minecraft mode.
3. **Rendering and placement:** Draw obstacle meshes and a ghost using the same
   generated dimensions as collision. Raycast or project from the camera onto
   ground to choose a placement pose.
4. **Editor controls and UI:** Add palette, selection, transform controls,
   edit/test toggle, and undo/redo. Hide these in normal gameplay.
5. **Persistence and local playtest:** Save under a dedicated user-data
   directory; load by map, verify placement and grind feel in a local match.
6. **Multiplayer:** Add host-owned layout replication, networked skater state,
   late joins, and two-client playtests.
7. **Public release:** Publish the source on the user's GitHub with attribution
   and no game files. Publish a Windows build only after a real multiplayer
   playtest and packaging review.

## Scope decisions for later

Publishing parks to a shared catalog, imported custom meshes, and a full
terrain sculptor can follow after the hosted multiplayer loop works. The first
version uses original obstacle meshes so it can be built without any new
game-asset extraction or distribution.
