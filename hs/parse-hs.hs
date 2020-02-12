{-# LANGUAGE BangPatterns #-}

import FastString
import Lexer (ParserFlags (..), ParseResult (..), P (..), mkPStatePure)
import Parser
import SrcLoc
import StringBuffer (hGetStringBuffer, StringBuffer)

import System.CPUTime (getCPUTime)
import System.Environment (getArgs)

runParser :: ParserFlags -> StringBuffer -> P a -> ParseResult a
runParser pflags buf parser = unP parser ps
  where
    filename = "<parse-module>"
    loc = mkRealSrcLoc (mkFastString filename) 1 1
    ps = mkPStatePure pflags buf loc

main :: IO ()
main = do
    [file] <- getArgs
    buf <- hGetStringBuffer file

    let pflags = ParserFlags
          { pWarningFlags = undefined
          , pThisPackage = undefined
          , pExtsBitmap = 0
          }

    t1 <- getCPUTime
    let success = case runParser pflags buf parseModule of
          POk _ _ -> True
          PFailed _ -> False
    t2 <- getCPUTime

    if success then
      putStrLn "Parse success"
    else
      putStrLn "Parse fail"

    putStrLn (showI ((t2 - t1)) ++ " ps")

showI :: Integer -> String
showI i = add_commas s (if m == 0 then 3 else m)
  where
    s = show i
    l = length s
    m = l `mod` 3

    add_commas [] 0 = ""
    add_commas cs 0 = ',' : add_commas cs 3
    add_commas (c : cs) n = c : add_commas cs (n - 1)
    add_commas cs n = error ("Bug in showI, cs=" ++ show cs ++ ", n=" ++ show n)
