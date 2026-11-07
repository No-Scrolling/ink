declare const __inkDomainToASCII: (domain: string) => string | null;

// The tr46 surface used by whatwg-url: nontransitional processing, CheckBidi and
// CheckJoiners, without CheckHyphens, STD3 rules or DNS-length verification.
// Every ASCII code point is valid with these options; only A–Z are mapped.
// whatwg-url rejects empty results and forbidden domain code points afterwards.
export function toASCII(domain: string): string | null {
  if (/^[\x00-\x7F]*$/.test(domain) && !/(?:^|\.)xn--/i.test(domain)) {
    return domain.toLowerCase();
  }
  return __inkDomainToASCII(domain);
}
