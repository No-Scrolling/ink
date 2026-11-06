// Pass this file's contents to the tldraw exec tool for an open board.
// Reimporting the same named frame updates its image and retains its position.
const name = __INK_NAME__;
const dataUrl = __INK_IMAGE__;
const assetId = 'asset:ink-__INK_ID__';
const frameId = 'shape:ink-__INK_ID__';
const imageId = 'shape:ink-__INK_ID__-image';
const bytes = Uint8Array.from(atob(dataUrl.split(',')[1]), c => c.charCodeAt(0));
const asset = {
  id: assetId, type: 'image', typeName: 'asset', meta: {},
  props: { name, src: dataUrl, w: __INK_WIDTH__, h: __INK_HEIGHT__, mimeType: 'image/png', isAnimated: false },
};
const uploaded = await editor.uploadAsset(asset, new File([bytes], `${name}.png`, { type: 'image/png' }), signal);
asset.props.src = uploaded.src;
asset.meta = uploaded.meta ?? {};
if (editor.getAsset(assetId)) editor.updateAssets([asset]);
else editor.createAssets([asset]);
const frame = editor.getShape(frameId);
if (frame) {
  editor.setCurrentPage(editor.getAncestorPageId(frame));
  editor.updateShape({ id: frameId, type: 'frame', props: { w: __INK_WIDTH__, h: __INK_HEIGHT__, name } });
} else {
  const bounds = editor.getCurrentPageBounds();
  editor.createShapes([{ id: frameId, type: 'frame', x: bounds ? bounds.maxX + 80 : 0, y: bounds ? bounds.minY : 0,
    props: { w: __INK_WIDTH__, h: __INK_HEIGHT__, name } }]);
}
const image = { id: imageId, type: 'image', parentId: frameId, x: 0, y: 0,
  props: { assetId, w: __INK_WIDTH__, h: __INK_HEIGHT__, altText: name } };
if (editor.getShape(imageId)) editor.updateShape(image);
else editor.createShape(image);
editor.select(frameId);
editor.zoomToSelection();
return { name, pageId: editor.getCurrentPageId(), width: __INK_WIDTH__, height: __INK_HEIGHT__, updated: !!frame };
