#include "memdeck.h"

static const char *rank_names[] = {
    "", "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"
};

static const char *rank_full[] = {
    "", "Ace", "Two", "Three", "Four", "Five", "Six",
    "Seven", "Eight", "Nine", "Ten", "Jack", "Queen", "King"
};

static const char *suit_names[] = { "Spades", "Hearts", "Clubs", "Diamonds" };
static const char *suit_letters[] = { "S", "H", "C", "D" };
static const char *suit_symbols[] = { "\xe2\x99\xa0", "\xe2\x99\xa5", "\xe2\x99\xa3", "\xe2\x99\xa6" };

int card_parse(const char *s, Card *c)
{
    if (!s || !c) return -1;

    const char *p = s;
    while (*p == ' ') p++;

    /* parse rank */
    if (*p == 'A' || *p == 'a') { c->rank = 1; p++; }
    else if (*p == 'J' || *p == 'j') { c->rank = 11; p++; }
    else if (*p == 'Q' || *p == 'q') { c->rank = 12; p++; }
    else if (*p == 'K' || *p == 'k') { c->rank = 13; p++; }
    else if (*p == '1' && *(p+1) == '0') { c->rank = 10; p += 2; }
    else if (*p >= '2' && *p <= '9') { c->rank = *p - '0'; p++; }
    else return -1;

    /* parse suit */
    if (*p == 'S' || *p == 's') c->suit = SUIT_SPADES;
    else if (*p == 'H' || *p == 'h') c->suit = SUIT_HEARTS;
    else if (*p == 'C' || *p == 'c') c->suit = SUIT_CLUBS;
    else if (*p == 'D' || *p == 'd') c->suit = SUIT_DIAMONDS;
    else return -1;

    return 0;
}

void card_code(const Card *c, char *buf, int buflen)
{
    snprintf(buf, buflen, "%s%s", rank_names[c->rank], suit_letters[c->suit]);
}

void card_display(const Card *c, char *buf, int buflen)
{
    snprintf(buf, buflen, "%s%s", rank_names[c->rank], suit_symbols[c->suit]);
}

const char *card_rank_name(int rank)
{
    if (rank < 1 || rank > 13) return "?";
    return rank_full[rank];
}

const char *card_suit_name(int suit)
{
    if (suit < 0 || suit > 3) return "?";
    return suit_names[suit];
}

const char *card_suit_symbol(int suit)
{
    if (suit < 0 || suit > 3) return "?";
    return suit_symbols[suit];
}

int card_color_pair(const Card *c)
{
    switch (c->suit) {
    case SUIT_HEARTS:   return CP_HEARTS;
    case SUIT_DIAMONDS: return CP_DIAMONDS;
    case SUIT_SPADES:   return CP_SPADES;
    case SUIT_CLUBS:    return CP_CLUBS;
    default:            return CP_NORMAL;
    }
}

int card_equal(const Card *a, const Card *b)
{
    return a->rank == b->rank && a->suit == b->suit;
}

int card_matches_filter(const Card *c, int filter)
{
    switch (filter) {
    case FILTER_ALL:      return 1;
    case FILTER_BLACK:    return c->suit == SUIT_SPADES || c->suit == SUIT_CLUBS;
    case FILTER_RED:      return c->suit == SUIT_HEARTS || c->suit == SUIT_DIAMONDS;
    case FILTER_HEARTS:   return c->suit == SUIT_HEARTS;
    case FILTER_SPADES:   return c->suit == SUIT_SPADES;
    case FILTER_CLUBS:    return c->suit == SUIT_CLUBS;
    case FILTER_DIAMONDS: return c->suit == SUIT_DIAMONDS;
    case FILTER_FACE:     return c->rank >= 11;
    case FILTER_NUMBERS:  return c->rank >= 2 && c->rank <= 10;
    default:              return 1;
    }
}
