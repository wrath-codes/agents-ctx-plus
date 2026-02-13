Sample RST
==========

Overview paragraph with a ``literal`` and a `target-name`_ and a standalone URL
https://example.com and **strong** plus *emphasis* and :py:func:`json.loads`.

.. _target-name: https://example.com/docs

.. |brand| replace:: Zenith

Using a substitution |brand| and a broken `missing-ref`_.

.. note::
   :class: callout

   This is a note directive body.

.. code-block:: python
   :linenos:

   print("hello")

.. include:: included.rst

Usage
-----

- First bullet
- Second bullet

1. First enum
2. Second enum

Term
  Definition paragraph.

.. [#] Footnote body text.

See footnote [#]_ and cite [CIT2001]_.

.. [CIT2001] Citation body text.

+-----------+-------------+
| Name      | Value       |
+===========+=============+
| project   | zenith      |
+-----------+-------------+
