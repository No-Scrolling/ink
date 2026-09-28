import { LinkifyIt } from "linkify-it";

let linkify: LinkifyIt | undefined;

export function textLinks(text: string) {
  linkify ??= new LinkifyIt({ fuzzyLink: true });
  return (linkify.match(text) || []).map(match => ({
    start: match.index,
    end: match.lastIndex,
    url: !match.schema
      ? match.url.replace(/^http:/, "https:") : match.url,
  }));
}
