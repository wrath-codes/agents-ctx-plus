module Zenith.Sample
  ( Status(..)
  , Widget(..)
  , UserId(..)
  , version
  , mkWidget
  , classify
  , onChange
  , hs_render
  ) where

import qualified Data.Text as T
import Data.Maybe (fromMaybe)
import Foreign.C.String (CString)
import Foreign.C.Types (CInt)

class Renderable a where
  render :: a -> T.Text

data Status
  = New
  | Ready
  | Done
  deriving (Eq, Show)

data Widget = Widget
  { widgetId :: Int
  , widgetName :: T.Text
  }
  deriving (Eq, Show)

newtype UserId = UserId Int
  deriving (Eq, Ord, Show)

version :: Int
version = 1

mkWidget :: Int -> T.Text -> Widget
mkWidget wid name = Widget { widgetId = wid, widgetName = name }

classify :: Status -> T.Text
classify New = "new"
classify Ready = "ready"
classify Done = "done"

onChange :: Widget -> IO ()
onChange _ = pure ()

foreign import ccall "puts" c_puts :: CString -> IO CInt
foreign export ccall hs_render :: Widget -> IO ()

hs_render :: Widget -> IO ()
hs_render _ = pure ()
