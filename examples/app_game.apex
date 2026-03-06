// Application: Number Guessing Game
// A simple game demonstrating Apex features

import std.io.*;

// Player class
class Player {
    name: String;
    mut score: Integer;
    mut gamesPlayed: Integer;
    mut totalGuesses: Integer;

    constructor(name: String) {
        this.name = name;
        this.score = 0;
        this.gamesPlayed = 0;
        this.totalGuesses = 0;
    }

    function getName(): String {
        return this.name;
    }

    function getScore(): Integer {
        return this.score;
    }

    function addScore(points: Integer): None {
        this.score = this.score + points;
        return None;
    }

    function recordGame(guesses: Integer): None {
        this.gamesPlayed = this.gamesPlayed + 1;
        this.totalGuesses = this.totalGuesses + guesses;
        return None;
    }

    function getAverageGuesses(): Float {
        if (this.gamesPlayed == 0) {
            return 0.0;
        }
        return to_float(this.totalGuesses) / to_float(this.gamesPlayed);
    }

    function displayStats(): None {
        println("");
        println("=== Player Stats ===");
        println("Name: {this.name}");
        println("Score: {this.score}");
        println("Games Played: {this.gamesPlayed}");
        return None;
    }
}

// Game state class
class GameState {
    mut targetNumber: Integer;
    mut guessCount: Integer;
    mut isActive: Boolean;
    minRange: Integer;
    maxRange: Integer;

    constructor(minVal: Integer, maxVal: Integer) {
        this.minRange = minVal;
        this.maxRange = maxVal;
        this.targetNumber = 0;
        this.guessCount = 0;
        this.isActive = false;
    }

    function startGame(target: Integer): None {
        this.targetNumber = target;
        this.guessCount = 0;
        this.isActive = true;
        println("");
        println("New game started!");
        println("Guess a number between {this.minRange} and {this.maxRange}");
        return None;
    }

    function makeGuess(guess: Integer): Integer {
        // Returns: 0 = correct, -1 = too low, 1 = too high
        this.guessCount = this.guessCount + 1;

        if (guess < this.targetNumber) {
            return -1;
        }
        if (guess > this.targetNumber) {
            return 1;
        }
        return 0;
    }

    function getGuessCount(): Integer {
        return this.guessCount;
    }

    function endGame(): None {
        this.isActive = false;
        return None;
    }

    function isGameActive(): Boolean {
        return this.isActive;
    }
}

// Scoring module
module Scoring {
    function calculateScore(guesses: Integer, maxRange: Integer): Integer {
        // Fewer guesses = higher score
        // Perfect score if guessed in log2(range) or less
        baseScore: Integer = 1000;
        penalty: Integer = guesses * 100;
        score: Integer = baseScore - penalty;

        if (score < 100) {
            return 100;  // Minimum score
        }
        return score;
    }

    function getRank(score: Integer): String {
        if (score >= 900) {
            return "Legendary";
        }
        if (score >= 700) {
            return "Expert";
        }
        if (score >= 500) {
            return "Skilled";
        }
        if (score >= 300) {
            return "Novice";
        }
        return "Beginner";
    }
}

// Hint generator
class HintGenerator {
    mut hintsUsed: Integer;

    constructor() {
        this.hintsUsed = 0;
    }

    function getHintCount(): Integer {
        return this.hintsUsed;
    }

    function generateHint(target: Integer, guess: Integer): String {
        this.hintsUsed = this.hintsUsed + 1;
        diff: Integer = Math.abs(target - guess);

        if (diff > 50) {
            return "Very far!";
        }
        if (diff > 20) {
            return "Getting warmer...";
        }
        if (diff > 10) {
            return "Warm!";
        }
        if (diff > 5) {
            return "Hot!";
        }
        return "On fire! Very close!";
    }

    function reset(): None {
        this.hintsUsed = 0;
        return None;
    }
}

// Difficulty settings
module Difficulty {
    function easyMin(): Integer {
        return 1;
    }

    function easyMax(): Integer {
        return 10;
    }

    function mediumMin(): Integer {
        return 1;
    }

    function mediumMax(): Integer {
        return 50;
    }

    function hardMin(): Integer {
        return 1;
    }

    function hardMax(): Integer {
        return 100;
    }

    function getName(maxRange: Integer): String {
        if (maxRange <= 10) {
            return "Easy";
        }
        if (maxRange <= 50) {
            return "Medium";
        }
        return "Hard";
    }
}

function printBanner(): None {
    println("");
    println("========================================");
    println("       NUMBER GUESSING GAME            ");
    println("========================================");
    return None;
}

function printSeparator(): None {
    println("----------------------------------------");
    return None;
}

function simulateGame(borrow mut player: Player, borrow mut game: GameState, borrow mut hints: HintGenerator): None {
    // Simulated game play (since we can't get real input)
    println("");
    printSeparator();
    println("Starting simulated game...");

    target: Integer = 42;  // Our "secret" number
    game.startGame(target);

    // Simulate guesses
    guesses: List<Integer> = List<Integer>();

    // Guess 1: 50 (too high)
    println("");
    println("Guess #1: 50");
    result1: Integer = game.makeGuess(50);
    println("Too high!");
    hint1: String = hints.generateHint(target, 50);
    println("Hint: {hint1}");

    // Guess 2: 25 (too low)
    println("");
    println("Guess #2: 25");
    result2: Integer = game.makeGuess(25);
    println("Too low!");
    hint2: String = hints.generateHint(target, 25);
    println("Hint: {hint2}");

    // Guess 3: 40 (too low)
    println("");
    println("Guess #3: 40");
    result3: Integer = game.makeGuess(40);
    println("Too low!");
    hint3: String = hints.generateHint(target, 40);
    println("Hint: {hint3}");

    // Guess 4: 45 (too high)
    println("");
    println("Guess #4: 45");
    result4: Integer = game.makeGuess(45);
    println("Too high!");
    hint4: String = hints.generateHint(target, 45);
    println("Hint: {hint4}");

    // Guess 5: 42 (correct!)
    println("");
    println("Guess #5: 42");
    result5: Integer = game.makeGuess(42);
    println("CORRECT! You found it!");

    game.endGame();
    totalGuesses: Integer = game.getGuessCount();

    // Calculate score
    score: Integer = Scoring__calculateScore(totalGuesses, 100);
    rank: String = Scoring__getRank(score);

    println("");
    printSeparator();
    println("GAME OVER!");
    println("Number of guesses: 5");
    println("Score earned: {score}");
    println("Rank: {rank}");
    println("Hints used: {hints.getHintCount()}");

    // Update player stats
    player.addScore(score);
    player.recordGame(totalGuesses);

    return None;
}

function main(): None {
    printBanner();

    // Create player
    println("");
    println("Welcome, Player!");
    mut player: Player = Player("Player1");
    println("Player created: Player1");

    // Show difficulty options
    println("");
    println("=== Difficulty Levels ===");
    println("Easy: 1-10 (good for beginners)");
    println("Medium: 1-50 (balanced challenge)");
    println("Hard: 1-100 (for experts)");

    // Create game with medium difficulty
    println("");
    println("Selected: Medium difficulty");
    mut game: GameState = GameState(Difficulty__mediumMin(), Difficulty__mediumMax());

    // Create hint generator
    mut hints: HintGenerator = HintGenerator();

    // Play a simulated game
    simulateGame(player, game, hints);

    // Show final player stats
    player.displayStats();

    // Play another round (simulated)
    println("");
    printSeparator();
    println("Playing another round...");

    hints.reset();
    mut game2: GameState = GameState(1, 100);

    // Quick simulated game
    target2: Integer = 73;
    game2.startGame(target2);

    println("");
    println("Guess #1: 50 - Too low!");
    game2.makeGuess(50);
    println("Guess #2: 75 - Too high!");
    game2.makeGuess(75);
    println("Guess #3: 70 - Too low!");
    game2.makeGuess(70);
    println("Guess #4: 73 - CORRECT!");
    game2.makeGuess(73);
    game2.endGame();

    score2: Integer = Scoring__calculateScore(4, 100);
    player.addScore(score2);
    player.recordGame(4);

    println("");
    println("Round 2 complete! Score: {score2}");

    // Final statistics
    println("");
    printSeparator();
    println("=== Final Statistics ===");
    player.displayStats();
    avg: Float = player.getAverageGuesses();
    println("Average guesses per game: ~4.5");

    println("");
    println("========================================");
    println("         Thanks for playing!           ");
    println("========================================");

    return None;
}
