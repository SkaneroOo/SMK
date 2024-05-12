select f1.name as file_1, 
       f2.name as file_2, 
       f3.name as file_3 
    from sets s 
    LEFT JOIN files f1 on s.id1 = f1.id
    LEFT JOIN files f2 on s.id2 = f2.id
    LEFT JOIN files f3 on s.id3 = f3.id
    ORDER BY random() limit 1;