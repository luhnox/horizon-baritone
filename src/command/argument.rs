// baritone.api.command.argument and .exception: what the words after a command mean, and the one
// kind of error a command raises.

/// A command that cannot go on, with the sentence that says why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError(pub String);

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

pub type CommandResult<T = ()> = Result<T, CommandError>;

/// A coordinate the way Baritone's RelativeCoordinate reads one: a number, or `~` and `~5` from the
/// block the player is in.
pub fn relative(word: &str, origin: i32) -> Option<i32> {
    relative_coordinate(word, f64::from(origin)).map(|v| v.floor() as i32)
}

/// RelativeCoordinate: `^(~?)([+-]?(?:\d+(?:\.\d*)?|\.\d+)([kKmM]?)|)$`. `10k` is 10000, `1.5m` 1500000, `~-5` five back.
pub fn relative_coordinate(word: &str, origin: f64) -> Option<f64> {
    let (is_relative, rest) = word.strip_prefix('~').map_or((false, word), |r| (true, r));
    let offset = if rest.is_empty() { 0.0 } else { scaled_number(rest)? };
    Some(if is_relative { origin + offset } else { offset })
}

fn scaled_number(word: &str) -> Option<f64> {
    let (number, scale) = match word.as_bytes().last()? {
        b'k' | b'K' => (&word[..word.len() - 1], 1e3),
        b'm' | b'M' => (&word[..word.len() - 1], 1e6),
        _ => (word, 1.0),
    };
    let digits = number.strip_prefix(['+', '-']).unwrap_or(number);
    let plain = digits.chars().all(|c| c.is_ascii_digit() || c == '.');
    let shaped = digits.matches('.').count() <= 1 && digits != ".";
    if digits.is_empty() || !plain || !shaped {
        return None;
    }
    Some(number.parse::<f64>().ok()? * scale)
}

pub fn looks_relative(word: &str) -> bool {
    relative_coordinate(word, 0.0).is_some()
}

/// The arguments of a command, taken one at a time.
#[derive(Debug, Clone)]
pub struct Args {
    words: Vec<String>,
    at: usize,
}

impl Args {
    pub fn parse(line: &str) -> Self {
        Self { words: line.split_whitespace().map(str::to_owned).collect(), at: 0 }
    }

    pub fn has_any(&self) -> bool {
        self.at < self.words.len()
    }

    pub fn has_at_least(&self, n: usize) -> bool {
        self.words.len() - self.at.min(self.words.len()) >= n
    }

    pub fn remaining(&self) -> usize {
        self.words.len().saturating_sub(self.at)
    }

    pub fn peek(&self) -> Option<&str> {
        self.words.get(self.at).map(String::as_str)
    }

    pub fn rest(&self) -> Vec<String> {
        self.words[self.at.min(self.words.len())..].to_vec()
    }

    pub fn rest_joined(&self) -> String {
        self.rest().join(" ")
    }

    /// The next word, or the error that says what was expected.
    pub fn string(&mut self, what: &str) -> CommandResult<String> {
        let w = self.words.get(self.at).cloned().ok_or_else(|| CommandError::new(format!("Missing {what}.")))?;
        self.at += 1;
        Ok(w)
    }

    pub fn optional_string(&mut self) -> Option<String> {
        let w = self.words.get(self.at).cloned();
        if w.is_some() {
            self.at += 1;
        }
        w
    }

    pub fn int(&mut self, what: &str) -> CommandResult<i32> {
        let w = self.peek().ok_or_else(|| CommandError::new(format!("Missing {what}.")))?.to_owned();
        let n = w.parse().map_err(|_| CommandError::new(format!("\"{w}\" is not a whole number ({what}).")))?;
        self.at += 1;
        Ok(n)
    }

    /// The next word as a whole number, if it is one; the word stays when it is not.
    pub fn try_int(&mut self) -> Option<i32> {
        let n = self.peek()?.parse().ok()?;
        self.at += 1;
        Some(n)
    }

    pub fn float(&mut self, what: &str) -> CommandResult<f64> {
        let w = self.peek().ok_or_else(|| CommandError::new(format!("Missing {what}.")))?.to_owned();
        let n = w.parse().map_err(|_| CommandError::new(format!("\"{w}\" is not a number ({what}).")))?;
        self.at += 1;
        Ok(n)
    }

    /// One of some words, any case.
    pub fn one_of(&mut self, options: &[&str], what: &str) -> CommandResult<String> {
        let w = self.string(what)?;
        options
            .iter()
            .find(|o| o.eq_ignore_ascii_case(&w))
            .map(|o| (*o).to_owned())
            .ok_or_else(|| CommandError::new(format!("\"{w}\" is not one of {} ({what}).", options.join(", "))))
    }

    pub fn has_exactly(&self, n: usize) -> bool {
        self.remaining() == n
    }

    pub fn require_max(&self, n: usize) -> CommandResult {
        if self.remaining() > n {
            return Err(CommandError::new(format!("Too many arguments (expected at most {n})")));
        }
        Ok(())
    }

    pub fn require_min(&self, n: usize) -> CommandResult {
        if self.remaining() < n {
            return Err(CommandError::new(format!("Not enough arguments (expected at least {n})")));
        }
        Ok(())
    }

    pub fn require_exactly(&self, n: usize) -> CommandResult {
        self.require_min(n)?;
        self.require_max(n)
    }

    /// Everything consumed so far, as typed.
    pub fn consumed_string(&self) -> String {
        self.words[..self.at.min(self.words.len())].join(" ")
    }

}

/// baritone.api.command.helpers.Paginator: how a list is cut into pages of eight.
#[derive(Clone, Copy, Debug)]
pub struct Paginator {
    pub len: usize,
    pub size: usize,
}

impl Paginator {
    pub fn new(len: usize) -> Self {
        Self { len, size: 8 }
    }

    pub fn max_page(&self) -> usize {
        if self.len == 0 { 1 } else { (self.len - 1) / self.size + 1 }
    }

    pub fn valid_page(&self, page: usize) -> bool {
        page > 0 && page <= self.max_page()
    }

    /// The entries of a page, as a range, and how many empty lines follow them.
    pub fn range(&self, page: usize) -> (std::ops::Range<usize>, usize) {
        let start = (page - 1) * self.size;
        let end = (start + self.size).min(self.len);
        let from = start.min(end);
        (from..end, self.size - (end - from))
    }

    /// The page asked for in the arguments (one at most), or the first.
    pub fn page_from(&self, args: &mut Args) -> CommandResult<usize> {
        args.require_max(1)?;
        if !args.has_any() {
            return Ok(1);
        }
        let word = args.string("a page")?;
        let page = word.parse::<usize>().ok().filter(|p| self.valid_page(*p));
        page.ok_or_else(|| CommandError::new(format!("Expected a valid page (1-{}), but got \"{word}\".", self.max_page())))
    }
}
