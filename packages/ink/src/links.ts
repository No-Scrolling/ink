import { LinkifyIt } from "linkify-it";

const linkify = new LinkifyIt({ fuzzyLink: true });

export function textLinks(text: string) {
  return (linkify.match(text) || []).map(match => ({
    start: match.index,
    end: match.lastIndex,
    url: !match.schema
      ? match.url.replace(/^http:/, "https:") : match.url,
  }));
}
